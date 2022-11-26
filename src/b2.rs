use std::{path::PathBuf, error::Error};

// TODO: File for B2 utils (backblaze/AWS support after compression :))

pub fn _try_upload_archive(archive_path: PathBuf, _remote_path: String, _remote_creds: String, _b2_or_aws: String) -> Result<PathBuf, Box<dyn Error>> {
    // TODO: Implement
    Ok(archive_path)
}
