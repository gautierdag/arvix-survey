use anyhow::Result;
use log::info;
use regex::Regex;
use crate::error::BibExtractError;

use std::fs;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;
use zip::ZipArchive;
use flate2::read::GzDecoder;
use tar::Archive;
use reqwest::blocking::Client;

use crate::latex::{Bibliography, ArxivPaper, citation};

/// Download and process an arXiv paper
pub fn download_arxiv_source(paper_id: &str) -> Result<ArxivPaper, BibExtractError> {
    let client = Client::new();
    let url = format!("https://arxiv.org/e-print/{}", paper_id);

    info!("Downloading source files from arXiv for paper: {}", paper_id);
    let response = client.get(&url).send().map_err(|e| BibExtractError::NetworkError(e))?;

    if !response.status().is_success() {
        return Err(BibExtractError::ApiError(format!("Failed to download source: HTTP {}", response.status())));
    }

    // Create temp directory to extract files
    let temp_dir = TempDir::new().map_err(|e| BibExtractError::IoError(e))?;
    let temp_path = temp_dir.path();

    // Save the downloaded source to a temporary file
    let mut source_file = tempfile::tempfile().map_err(|e| BibExtractError::IoError(e))?;
    let content = response.bytes().map_err(|e| BibExtractError::NetworkError(e))?;
    
    if content.is_empty() {
        return Err(BibExtractError::ApiError(format!("Received empty content from arXiv for paper ID: {}", paper_id)));
    }
    
    source_file.write_all(&content).map_err(|e| BibExtractError::IoError(e))?;
    source_file.seek(std::io::SeekFrom::Start(0)).map_err(|e| BibExtractError::IoError(e))?;
    
    // Extract the archive
    extract_archive(source_file, temp_path)?;
    
    // Find the main .tex file
    let main_tex_file = find_main_tex_file(temp_path)?;
    
    // Extract all LaTeX content
    let (full_content, _) = extract_all_latex_from_files(temp_path, &main_tex_file)?;
    
    // Find all .bbl files in the workspace
    let bbl_files = find_bbl_files(temp_path)?;
    
    // Parse bibliography
    let bibliography = Bibliography::parse_bibliography_files(&bbl_files)?;
    
    // Extract sections from the full content
    let sections = citation::extract_sections_from_latex(&full_content, &bibliography)?;

    // get title and authors from arvix/bibtex/id
    let bibtex_url = format!("https://arxiv.org/bibtex/{}", paper_id);
    let bibtex_response = client.get(&bibtex_url).send().map_err(|e| BibExtractError::NetworkError(e))?;
    if !bibtex_response.status().is_success() {
        return Err(BibExtractError::ApiError(format!("Failed to download BibTeX: HTTP {}", bibtex_response.status())));
    }
    let bibtex_content = bibtex_response.text().map_err(|e| BibExtractError::NetworkError(e))?;
    
    // Extract title and authors from the BibTeX content
    let title_re = Regex::new(r"title\s*=\s*\{([\s\S]*?)\}").unwrap();
    let authors_re = Regex::new(r"author\s*=\s*\{([\s\S]*?)\}").unwrap();
    let title = title_re.captures(&bibtex_content)
        .and_then(|cap| cap.get(1))
        .map_or("Unknown Title".to_string(), |m| m.as_str().trim().to_string());

    let authors = authors_re.captures(&bibtex_content)
        .and_then(|cap| cap.get(1))
        .map_or("Unknown Authors".to_string(), |m| m.as_str().trim().to_string());

    // shorten the title and authors if they are too long
    let max_length = 100;
    let title = if title.len() > max_length {
        format!("{}...", &title[..max_length - 3])
    } else {
        title
    };
    let authors: String = if authors.len() > max_length {
        format!("{}...", &authors[..max_length - 3])
    } else {
        authors
    };

    Ok(ArxivPaper {
        id: paper_id.to_string(),
        title: title,
        authors: authors,
        sections,
        bibliography,
        _temp_dir: temp_dir,
    })
}


/// Extract archive (supports ZIP and TAR.GZ)
fn extract_archive<R: Read + io::Seek>(mut archive: R, output_dir: &Path) -> Result<(), BibExtractError> {
    // Try to open as ZIP first
    match ZipArchive::new(&mut archive) {
        Ok(mut zip) => {
            info!("Extracting ZIP archive");
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).map_err(|e| BibExtractError::ZipError(e))?;
                let outpath = match file.enclosed_name() {
                    Some(path) => output_dir.join(path),
                    None => continue,
                };

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath).map_err(|e| BibExtractError::IoError(e))?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p).map_err(|e| BibExtractError::IoError(e))?;
                        }
                    }
                    let mut outfile = fs::File::create(&outpath).map_err(|e| BibExtractError::IoError(e))?;
                    io::copy(&mut file, &mut outfile).map_err(|e| BibExtractError::IoError(e))?;
                }
            }
            return Ok(());
        },
        Err(_) => {
            // Rewind the file
            archive.seek(SeekFrom::Start(0)).map_err(|e| BibExtractError::IoError(e))?;
            
            // Try as tar.gz
            info!("Trying to extract as TAR.GZ archive");
            let gz = GzDecoder::new(archive);
            let mut tar = Archive::new(gz);
            tar.unpack(output_dir).map_err(|e| BibExtractError::IoError(e))?;
            return Ok(());
        }
    }
}

/// Find all BBL files in a directory
pub fn find_bbl_files(dir: &Path) -> Result<Vec<PathBuf>, BibExtractError> {
    let bbl_files = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().is_file() && 
            entry.path().extension().map_or(false, |ext| ext == "bbl")
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();
    
    Ok(bbl_files)
}

/// Find the main LaTeX file in a directory
pub fn find_main_tex_file(dir: &Path) -> Result<PathBuf, BibExtractError> {
    // Look for common main file names
    let common_names = ["main.tex", "paper.tex", "article.tex", "manuscript.tex"];
    for name in &common_names {
        let path = dir.join(name);
        if path.exists() {
            return Ok(path);
        }
    }
    
    // If no common names found, look for any .tex file with \documentclass
    let tex_files: Vec<PathBuf> = WalkDir::new(dir)
        .max_depth(2)  // Don't go too deep
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().is_file() && 
            entry.path().extension().map_or(false, |ext| ext == "tex")
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();
    
    // Check for files with \documentclass
    for file in &tex_files {
        if let Ok(content) = fs::read_to_string(file).map_err(|e| BibExtractError::IoError(e)) {
            if content.contains(r"\documentclass") {
                return Ok(file.clone());
            }
        }
    }
    
    // If we still haven't found anything, just return the first .tex file
    if !tex_files.is_empty() {
        return Ok(tex_files[0].clone());
    }
    
    Err(BibExtractError::ApiError(format!("No LaTeX main file found in {:?}", dir)))
}

/// Extract all LaTeX content from files including handling \input commands
pub fn extract_all_latex_from_files(
    base_dir: &Path,
    main_tex_file: &Path,
) -> Result<(String, Vec<PathBuf>), BibExtractError> {
    let mut included_files = Vec::new();
    let mut processed_files = Vec::new();
    
    let content = extract_latex_content(
        base_dir,
        main_tex_file,
        &mut included_files,
        &mut processed_files,
    )?;
    
    Ok((content, included_files))
}

/// Recursive helper function to extract LaTeX content
fn extract_latex_content(
    base_dir: &Path,
    tex_file: &Path,
    included_files: &mut Vec<PathBuf>,
    processed_files: &mut Vec<PathBuf>,
) -> Result<String, BibExtractError> {
    // Avoid processing the same file twice
    if processed_files.iter().any(|p| p == tex_file) {
        return Ok(String::new());
    }
    
    // Mark this file as processed
    processed_files.push(tex_file.to_path_buf());
    
    // Add to included_files (excluding the main file which is the first one processed)
    if processed_files.len() > 1 {
        included_files.push(tex_file.to_path_buf());
    }
    
    // Read the file content
    let content = fs::read_to_string(tex_file).map_err(|e| BibExtractError::IoError(e))?;
    
    // Look for \input and \include commands
    let mut result = String::new();
    let input_re = Regex::new(r"\\(input|include)\{([^}]+)\}").map_err(|e| BibExtractError::ApiError(e.to_string()))?;
    
    let mut last_end = 0;
    for cap in input_re.captures_iter(&content) {
        let full_match = cap.get(0).unwrap();
        // Add the content before this match
        result.push_str(&content[last_end..full_match.start()]);
        last_end = full_match.end();
        
        // Extract the filename
        let filename = cap.get(2).unwrap().as_str();
        
        // Resolve the path
        if let Some(input_path) = resolve_input_path(base_dir, filename)? {
            // Recursively process the included file
            let included_content = extract_latex_content(
                base_dir,
                &input_path,
                included_files,
                processed_files,
            )?;
            // Add the included content
            result.push_str(&included_content);
        }
    }
    
    // Add any remaining content
    result.push_str(&content[last_end..]);
    
    Ok(result)
}

/// Resolve the path of an input file
pub fn resolve_input_path(base_dir: &Path, filename: &str) -> Result<Option<PathBuf>, BibExtractError> {
    // Check if the file exists as is
    let direct_path = base_dir.join(filename);
    if direct_path.exists() && direct_path.is_file() {
        return Ok(Some(direct_path));
    }
    
    // Try adding .tex extension if not present
    if !filename.ends_with(".tex") {
        let with_extension = format!("{}.tex", filename);
        let path_with_extension = base_dir.join(&with_extension);
        if path_with_extension.exists() && path_with_extension.is_file() {
            return Ok(Some(path_with_extension));
        }
    }
    
    // Not found
    Ok(None)
}


