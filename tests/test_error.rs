use bibextract::error::BibExtractError;
use serde_json::Error as SerdeJsonError;
use std::io::Error as IoError;
use zip::result::ZipError;
use anyhow::anyhow;

#[test]
fn test_network_error() {
    let reqwest_error = reqwest::blocking::get("http://nonexistent.url").unwrap_err();
    let error = BibExtractError::from(reqwest_error);
    assert!(format!("{}", error).contains("Network request failed"));
}

#[test]
fn test_json_parsing_error() {
    let json_error: SerdeJsonError = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let error = BibExtractError::from(json_error);
    assert_eq!(format!("{}", error), "Failed to parse JSON response: expected value at line 1 column 1");
}

#[test]
fn test_io_error() {
    let io_error = IoError::new(std::io::ErrorKind::NotFound, "file not found");
    let error = BibExtractError::from(io_error);
    assert_eq!(format!("{}", error), "I/O error: file not found");
}

#[test]
fn test_zip_error() {
    let zip_error = ZipError::FileNotFound;
    let error = BibExtractError::from(zip_error);
    assert_eq!(format!("{}", error), "ZIP archive error: specified file not found in archive");
}

#[test]
fn test_invalid_arxiv_id() {
    let error = BibExtractError::InvalidArxivId("123".to_string());
    assert_eq!(format!("{}", error), "Invalid arXiv ID: 123");
}

#[test]
fn test_no_paper_ids_provided() {
    let error = BibExtractError::NoPaperIdsProvided;
    assert_eq!(format!("{}", error), "No paper IDs provided");
}

#[test]
fn test_bibtex_parsing_error() {
    let error = BibExtractError::BibtexParsingError;
    assert_eq!(format!("{}", error), "Failed to parse BibTeX entry");
}

#[test]
fn test_api_error() {
    let error = BibExtractError::ApiError("Something went wrong with the API".to_string());
    assert_eq!(format!("{}", error), "API error: Something went wrong with the API");
}

#[test]
fn test_unknown_error() {
    let error = BibExtractError::Unknown;
    assert_eq!(format!("{}", error), "Unknown error");
}

#[test]
fn test_from_anyhow_error() {
    let anyhow_error = anyhow!("An unexpected error occurred");
    let error = BibExtractError::from(anyhow_error);
    assert_eq!(format!("{}", error), "Unknown error");
}
