use thiserror::Error;

#[derive(Error, Debug)]
pub enum BibExtractError {
    #[error("Network request failed: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Failed to parse JSON response: {0}")]
    JsonParsingError(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("ZIP archive error: {0}")]
    ZipError(#[from] zip::result::ZipError),

    #[error("Invalid arXiv ID: {0}")]
    InvalidArxivId(String),

    #[error("No paper IDs provided")]
    NoPaperIdsProvided,

    #[error("Failed to parse BibTeX entry")]
    BibtexParsingError,

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Unknown error")]
    Unknown,
}

impl From<anyhow::Error> for BibExtractError {
    fn from(_: anyhow::Error) -> Self {
        BibExtractError::Unknown
    }
}