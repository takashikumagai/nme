//! Detects a file's format from its path and opens the matching
//! [`crate::MetadataFile`] implementation.
//!
//! This is the single place that needs to change when a new format is
//! added: implement `MetadataFile` in a new module, then add one arm here.

use std::path::Path;

use crate::flac::FlacFile;
use crate::mp3::Mp3File;
use crate::pdf::PdfFile;
use crate::{MetadataFile, NmeError, Result};

pub fn open(path: &Path) -> Result<Box<dyn MetadataFile>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "pdf" => Ok(Box::new(PdfFile::open(path)?)),
        "mp3" => Ok(Box::new(Mp3File::open(path)?)),
        "flac" => Ok(Box::new(FlacFile::open(path)?)),
        "jpg" | "jpeg" => Err(NmeError::UnsupportedFormat(
            "JPEG (planned, not yet implemented)".to_string(),
        )),
        "" => Err(NmeError::UnsupportedFormat(format!(
            "{} (no file extension)",
            path.display()
        ))),
        other => Err(NmeError::UnsupportedFormat(other.to_string())),
    }
}
