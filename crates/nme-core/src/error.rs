use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum NmeError {
    #[error("unsupported or not-yet-implemented format: {0}")]
    UnsupportedFormat(String),

    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse PDF {path}: {source}")]
    PdfParse {
        path: PathBuf,
        #[source]
        source: lopdf::Error,
    },

    #[error("failed to save PDF {path}: {source}")]
    PdfSave {
        path: PathBuf,
        #[source]
        source: lopdf::Error,
    },

    #[error("failed to parse MP3 {path}: {source}")]
    Mp3Parse {
        path: PathBuf,
        #[source]
        source: id3::Error,
    },

    #[error("failed to save MP3 {path}: {source}")]
    Mp3Save {
        path: PathBuf,
        #[source]
        source: id3::Error,
    },
}

pub type Result<T> = std::result::Result<T, NmeError>;
