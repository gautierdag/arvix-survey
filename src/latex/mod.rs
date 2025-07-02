use log::{info};
use once_cell::sync::Lazy;
use regex::Regex;

pub mod bibliography;
pub mod citation;
pub mod parser;
pub mod verification;

// Re-export commonly used types
pub use bibliography::{Bibliography, BibEntry, BibEntryBuilder};
pub use citation::ExtractedSection;
pub use parser::{download_arxiv_source, find_bbl_files, extract_all_latex_from_files, resolve_input_path};

// Commonly used regex patterns compiled once
pub static CITE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\\(?:cite|citep|citet|citealp|citeauthor)\{([^}]+)\}").expect("Invalid citation regex pattern")
});
pub static ARXIV_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"arXiv:?\s*([0-9]+\.[0-9]+)").expect("Invalid arXiv ID regex pattern")
});
pub static ARXIV_KEY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([0-9]{4}\.[0-9]+)$").expect("Invalid arXiv key regex pattern")
});
pub static BIBTEX_ENTRY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"@([a-zA-Z]+)\{([^,]+),").expect("Invalid BibTeX entry regex pattern")
});
pub static BIBTEX_FIELD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"([a-zA-Z]+)\s*=\s*(?:\{((?:[^{}]|\{[^{}]*\})*)\}|"([^"]*)")"#).expect("Invalid BibTeX field regex pattern")
});

/// Helper function to clean text by removing punctuation and special characters
pub fn clean_text(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("_")
        .to_lowercase()
}

// ArXiv paper structure and functionality
pub struct ArxivPaper {
    pub id: String,                          // arXiv ID
    pub title: String,
    pub authors: String,
    pub sections: Vec<ExtractedSection>,     // extracted sections
    pub bibliography: Bibliography,          // parsed bibliography
    pub _temp_dir: tempfile::TempDir,            // Temporary directory (keep alive while the paper is used)
}

impl ArxivPaper {
    /// Verify bibliography entries using parallel processing for both sources (DBLP and arXiv simultaneously)
    pub fn verify_bibliography(&mut self) -> anyhow::Result<usize> {
        use log::info;
        use std::sync::{Arc, Mutex};
        use std::collections::HashMap;
        use rayon::prelude::*;

        info!("Verifying bibliography entries for paper {}", self.id);
        
        let keys: Vec<String> = self.bibliography.iter().map(|entry| entry.key.clone()).collect();
        let entries_count = keys.len();
        
        // Create shared result container to collect verified entries
        let verified_entries = Arc::new(Mutex::new(HashMap::new()));
        let verification_count = Arc::new(Mutex::new(0usize));
        
        // Process entries in parallel
        keys.par_iter().for_each(|key| {
            if let Some(entry) = self.bibliography.get(key) {
                let mut entry_clone = entry.clone();
                
                // Create a temporary bibliography instance to avoid borrowing issues
                let temp_bib = Bibliography::new();
                
                match temp_bib.verify_entry(&mut entry_clone) {
                    Ok(true) => {
                        // Successfully verified
                        let mut count = verification_count.lock().unwrap();
                        *count += 1;
                        
                        // Store verified entry
                        let mut entries = verified_entries.lock().unwrap();
                        entries.insert(key.clone(), entry_clone);
                        
                        info!("Verified entry: {} (progress: {}/{})", key, *count, entries_count);
                    },
                    Ok(false) => {
                        info!("Could not verify entry: {}", key);
                    },
                    Err(e) => {
                        log::warn!("Error verifying entry {}: {}", key, e);
                    }
                }
            }
        });
        
        // Update the original entries with verified data
        let verified = verified_entries.lock().unwrap();
        for (key, verified_entry) in verified.iter() {
            if let Some(entry) = self.bibliography.entries.get_mut(key) {
                *entry = verified_entry.clone();
            }
        }
        
        let verified_count = *verification_count.lock().unwrap();
        info!("Verified {}/{} bibliography entries using dual source parallel processing", 
              verified_count, entries_count);
        
        Ok(verified_count)
    }
}