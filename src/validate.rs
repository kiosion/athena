use std::{fs, path::PathBuf, error::Error};

// Validates input dir / file exists
pub fn input(input: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    if !input.exists() {
        return Err("Specified file or directory does not exist".into());
    }
    Ok(input)
}

// Validates output dir is valid
pub fn output(output: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    // If output doesn't exist, we should prompt the user whether to create it
    if !output.exists() {
        if output.is_file() {
            if output.parent().unwrap().exists() {
                return Ok(output);
            }
        }
        eprintln!("Output directory does not exist: '{}'", output.display());
        eprint!("Create it? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        if input.trim().to_lowercase() == "y" {
            fs::create_dir(&output).unwrap();
        } else {
            return Err("Output directory does not exist".into())
        }
    }
    Ok(output)
}

// Validates the generated archive file to ensure files were written and archive is a valid tar.gzip file
pub fn archive(out: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    if !out.exists() {
        return Err("Failed to write archive".into());
    }
    if out.metadata()?.len() == 0 {
        fs::remove_file(&out)?;
        return Err("No files were processed".into());
    }
    let mut file = std::fs::File::open(&out)?;
    let mut buf = [0; 2];
    std::io::Read::read_exact(&mut file, &mut buf)?;
    if buf != [0x1f, 0x8b] {
        fs::remove_file(&out)?;
        return Err("Invalid archive".into());
    }
    Ok(out)
}
