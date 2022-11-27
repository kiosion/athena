use std::{time::Duration, path::PathBuf, fs, process, error};
use clap::Parser;
use flate2::{write::GzEncoder, Compression};
use futures::future::{BoxFuture, FutureExt};
use indicatif::ProgressBar;
use file_owner::PathExt;
use tokio::signal::ctrl_c;

mod validate;
mod utils;
mod b2;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'i', long = "src")]
    src: String,
    #[arg(short = 'o', long = "dest")]
    dest: String,
    #[arg(short = 'c', long = "compress")]
    compress: bool,
    #[arg(short = 'u', long = "upload")]
    upload: bool,
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

// Handle early SIGINT / SIGTERM
async fn handle_term() {
    // TODO: Properly handle termination by sending a signal to any running fns
    eprintln!("Terminating...");
    process::exit(0);
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
    let output_path = match validate::output(PathBuf::from(&args.dest)) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let options = utils::Options {
        verbose: args.verbose,
        upload: args.upload,
        compression: args.compress,
        input_path,
        output_path,
    };

    tokio::spawn(async move {
        ctrl_c().await.unwrap();
        handle_term().await;
    });

    let spinner = utils::construct_spinner();
    spinner.enable_steady_tick(Duration::from_millis(150));
    println!("");
    spinner.set_message("Processing files...");

    let handle = tokio::task::spawn_blocking({
        let path = options.input_path.clone();
        move || {
            process_input(path)
    }}).await.unwrap();

    match handle.await {
        Ok(files) => {
            spinner.finish_and_clear();
            if options.verbose {
                println!(
                    "{} {} processed",
                    files.len(),
                    if files.len() == 1 { "file" } else { "files" }
                );
            }

            let progress_bar = utils::construct_progress(files.len() as u64);
            progress_bar.set_message(format!(
                "{m} {f} {t}...",
                m = if options.compression { "Compressing" } else { "Writing" },
                f = files.len(),
                t = if files.len() > 1 { "files" } else { "file" }
            ));

            let handle = tokio::task::spawn_blocking({
                let options = options.to_owned();
                let files = files.to_owned();
                move || {
                construct_archive(files, options, progress_bar)
            }}).await.unwrap();

            match handle.await {
                Ok(archive_buf) => {
                    // if !options.upload,
                    print_done(files, archive_buf, &options.compression);
                    // else call upload_archive
                },
                Err(e) => {
                    eprintln!("Error: {}", e);
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

fn print_done(input_files: Vec<PathBuf>, archive_buf: PathBuf, compression: &bool) {
    let mut input_size = 0.;
    for file in input_files {
        input_size += file.metadata().unwrap().len() as f64;
    }
    let mut out_size = archive_buf.metadata().unwrap().len() as f64;
    let mut size_unit = "B";

    match input_size {
        i if i > 1000000000. => {
            input_size /= 1000000000.;
            out_size /= 1000000000.;
            size_unit = "GB";
        },
        i if i > 1000000. => {
            input_size /= 1000000.;
            out_size /= 1000000.;
            size_unit = "MB";
        },
        i if i > 1000. => {
            input_size /= 1000.;
            out_size /= 1000.;
            size_unit = "KB";
        },
        _ => (),
    }

    // just &bool for now so this feels a bit odd but whatev
    match compression {
        true => {
            let reduction = (((out_size / input_size) * 100.0) * 100.0).round() / 100.0;
            out_size = (out_size * 100.0).round() / 100.0;
            println!(
                "Successfully wrote {size}{unit} to {loc} (deflated {percent}%)",
                size = out_size,
                unit = size_unit,
                loc = archive_buf.display(),
                percent = reduction
            );
        },
        _ => {
            println!(
                "Successfully wrote {size}{unit} to {loc}",
                size = out_size,
                unit = size_unit,
                loc = archive_buf.display()
            );
        },
    };
    process::exit(0);
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

// Fn to handle adding files to the dest archive, and compressing them if specified
async fn construct_archive(paths: Vec<PathBuf>, options: utils::Options, progress: ProgressBar) -> Result<PathBuf, Box<dyn error::Error>> {
    let input_path = options.input_path.clone();
    let output_path = options.output_path.clone();

    // Unless overridden, default filename is the current time (YYYYMMDDHHMMSS).tar.gz plus the filename, or last directory name
    let mut file_name = if output_path.is_file() {
        output_path.file_name().unwrap().to_str().unwrap().to_string()
    } else {
        chrono::Local::now().format(&format!("%Y%m%d%H%M-{}", input_path.file_name().unwrap().to_str().unwrap().to_string())).to_string()
    };
    let extension = match options.compression {
        true => "tgz",
        _ => "tar",
    };
    file_name.push_str(&format!(".{}", extension));

    let file_path = output_path.clone().join(&file_name);
    if file_path.exists() {
        let overwrite = utils::prompt_user(format!("File {} already exists in {}", &file_name, &output_path.display()), "Overwrite?".to_string(), Some(false));

        if !overwrite {
            process::exit(0);
        }
    }

    let archive_file = fs::File::create(&file_path).unwrap();

    let mut archive = tar::Builder::new(match &options.compression {
        true => Box::new(GzEncoder::new(archive_file, Compression::best())) as Box<dyn std::io::Write>,
        _ => Box::new(archive_file) as Box<dyn std::io::Write>,
    });
  
    progress.enable_steady_tick(Duration::from_millis(150));
    let input_path_only = get_inp_path_only(&input_path);
    let mut files_processed = 0;
    for path in paths {
        let rel_path = path.strip_prefix(&input_path_only).unwrap();
        if path.symlink_metadata().unwrap().file_type().is_symlink() {
            // Add symlink to archive, with header, rel path in archive, and target path on sys
            let mut header = tar::Header::new_gnu();
            header.set_uid(path.owner().unwrap().id() as u64);
            header.set_gid(path.group().unwrap().id() as u64);
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            archive.append_link(&mut header, rel_path, path.read_link().unwrap().to_str().unwrap())?;
        } else {
            // Since set_path() using this lib can't take pathnames > 255 bytes, use
            // its append_path_with_name method to insert the pathname at the same time as the file content
            archive.append_path_with_name(&path, rel_path)?;
        }
        files_processed += 1;
        progress.set_position(files_processed as u64);
    }
    archive.finish()?;

    match validate::archive(file_path) {
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
// TODO: Find another way to achieve this without storing all PathBufs in memory, this could be a problem for
// dirs with a lot of files (although at least up to 100k files it seems to be fine so ehhhhh)
fn process_input(input_path: PathBuf) -> BoxFuture<'static, Result<Vec<PathBuf>, Box<dyn error::Error + Send + Sync>>> {
    async move {
        if input_path.is_symlink() || input_path.is_file() {
            Ok(vec![input_path])
        } else {
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
        }
    }.boxed()
}
