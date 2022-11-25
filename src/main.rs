use std::time::Duration;
use std::{fs, process, error};
use std::path::PathBuf;
use clap::Parser;
use futures::future::{BoxFuture, FutureExt};
use indicatif::{ProgressBar};

mod validate;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'i', long = "src")]
    src: String,
    #[arg(short = 'o', long = "dest")]
    dest: String,
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();

    let input_path = match validate::input(PathBuf::from(&args.src)) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        },
    };
    // println!("Input path: {}", input_path.display());

    let output_path = match validate::output(PathBuf::from(&args.dest)) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };
    // println!("Output path: {}", output_path.display());

    let spinner = utils::construct_spinner();
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Processing files...");

    let handle = tokio::task::spawn_blocking({
        let path = input_path.clone();
        move || {
            process_input(path)
    }}).await.unwrap();

    match handle.await {
        Ok(files) => {
            spinner.finish_and_clear();
            println!("{} file(s) processed", files.len());

            let progress_bar = utils::construct_progress(files.len() as u64);
            progress_bar.set_message(format!("Compressing {f} {t}...", f = files.len(), t = if files.len() > 1 { "files" } else { "file" }));

            // Here we pass the Vec of PathBufs to compress_files(), as well as the progress bar
            // so it can be iterated as files are handled
            let handle = tokio::task::spawn_blocking(move || {
                compress_files(files, input_path, output_path, progress_bar)
            }).await.unwrap();

            match handle.await {
                Ok(archive_buf) => {
                    println!("Archive written to {}", archive_buf.display());
                    process::exit(0);
                },
                Err(e) => {
                    eprintln!("Error adding files: {}", e);
                    process::exit(1);
                },
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

// Used in getting the relative path of files added to the archive
// so that the archive can be extracted to the same directory structure
fn get_inp_path_only(path: &PathBuf) -> String {
    if path.is_file() {
        path.parent().unwrap().to_str().unwrap().to_string()
    } else {
        path.to_str().unwrap().to_string()
    }
}

// Main fn that handles the compression of files and writing of the archive File
async fn compress_files(paths: Vec<PathBuf>, input_dir: PathBuf, output_dir: PathBuf, progress: ProgressBar) -> Result<PathBuf, Box<dyn error::Error>> {
    // Unless overridden, default filename is the current time (YYYYMMDDHHMMSS).tar.gz plus the filename, or last directory name
    // Just to make sure we don't overwrite an existing archive, we'll append a number to the end of the filename if it already exists
    let mut filename = chrono::Local::now().format(&format!("%Y%m%d%H%M-{}", input_dir.file_name().unwrap().to_str().unwrap().to_string())).to_string();
    let mut i = 0;
    loop {
        let mut path = output_dir.clone();
        path.push(format!("{}.tar.gz", filename));
        if !path.exists() {
            break;
        }
        i += 1;
        filename = format!("{}_{}", filename, i);
    }

    let archive_file = fs::File::create(output_dir.join(format!("{}.tar.gz", filename))).unwrap();
    let mut encoder = flate2::write::GzEncoder::new(archive_file, flate2::Compression::default());
    let mut archive = tar::Builder::new(&mut encoder);
    let inp_path = get_inp_path_only(&input_dir);

    // Set up progress bar's refresh rate
    progress.enable_steady_tick(Duration::from_millis(100));
    let mut files_processed = 0;

    // Event stream rt
    // let rt = tokio::runtime::Builder::new_current_thread()
    //     .enable_time()
    //     .build()
    //     .expect("failed to create runtime");

    for path in paths {
        // Get relative path of file by removing the input path from the file path
        let rel_path = path.strip_prefix(&inp_path).unwrap();
        // If path is symlink, add to archive as symlink and don't follow it
        if path.symlink_metadata().unwrap().file_type().is_symlink() {
            // get header from symlink
            let mut header = tar::Header::new_gnu();
            // set header's path to relative path
            header.set_path(rel_path)?;
            // set header's link_name to the symlink's target
            // header.set_link_name(path.read_link().unwrap().to_str().unwrap())?;
            // set entryType to symlink
            header.set_entry_type(tar::EntryType::Symlink);
            archive.append_link(&mut header, rel_path, path.read_link().unwrap().to_str().unwrap())?;
            // archive.append_symlink(rel_path, path.read_link().unwrap())?;
        } else {
            // Add to archive and step progress bar
            // TODO: This needs to be done manually, since
            // append_file consistently fails with long pathnames
            let mut file = fs::File::open(&path).unwrap();
            archive.append_file(rel_path, &mut file).unwrap();
        }
        files_processed += 1;
        progress.set_position(files_processed as u64);
    }

    archive.finish().unwrap();

    // Hand off archive_file to process_output() as PathBuf to check validity
    match validate::archive(output_dir, &filename) {
        Ok(path) => {
            progress.finish_and_clear();
            Ok(path)
        },
        Err(e) => {
            progress.finish_with_message("Failed");
            Err(e)
        },
    }
}

// Checks over the given input directory, counting files and subdirs and returning a BoxFuture that resolves to a Vec of PathBufs
// of the absolute paths to all files in the given directory
fn process_input(input_path: PathBuf) -> BoxFuture<'static, Result<Vec<PathBuf>, Box<dyn error::Error + Send + Sync>>> {
    async move {
        // If input is a symlink, add as if it were a file and do not recurse
        if input_path.is_symlink() {
            return Ok(vec![input_path]);
        }
        // If file, return a Vec containing the file path
        if input_path.is_file() {
            return Ok(vec![input_path]);
        }
        // Else, crawl the directory and recursively call self on each file
        let mut files = Vec::new();
        for entry in fs::read_dir(input_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // println!("Processing directory: {}", path.display());
                files.append(&mut process_input(path).await?);
            } else {
                files.push(path);
            }
        }
        Ok(files)
    }.boxed()
}
