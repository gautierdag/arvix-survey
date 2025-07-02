use log::info;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::latex::{clean_text, CITE_REGEX, ARXIV_ID_REGEX, ARXIV_KEY_REGEX};
use crate::error::BibExtractError;


/// Custom bibliography entry structure
#[derive(Debug, Clone)]
pub struct BibEntry {
    pub key: String,
    pub entry_type: String,
    pub fields: HashMap<String, String>,
}

/// Builder for BibEntry to allow for cleaner creation
pub struct BibEntryBuilder {
    key: String,
    entry_type: String,
    fields: HashMap<String, String>,
}

impl BibEntryBuilder {
    /// Create a new BibEntryBuilder with the required key and entry type
    pub fn new(key: impl Into<String>, entry_type: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            entry_type: entry_type.into(),
            fields: HashMap::new(),
        }
    }

    /// Add a field to the BibEntry
    pub fn field(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(field.into(), value.into());
        self
    }

    /// Add multiple fields from an iterator of (field, value) pairs
    pub fn fields<I, K, V>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (field, value) in fields {
            self.fields.insert(field.into(), value.into());
        }
        self
    }

    /// Build the BibEntry
    pub fn build(self) -> BibEntry {
        BibEntry {
            key: self.key,
            entry_type: self.entry_type,
            fields: self.fields,
        }
    }
}

impl BibEntry {
    pub fn new(key: String, entry_type: String) -> Self {
        Self {
            key,
            entry_type,
            fields: HashMap::new(),
        }
    }
    
    /// Create a new BibEntry using the builder pattern
    pub fn builder(key: impl Into<String>, entry_type: impl Into<String>) -> BibEntryBuilder {
        BibEntryBuilder::new(key, entry_type)
    }
    
    pub fn set(&mut self, field: &str, value: String) {
        self.fields.insert(field.to_string(), value);
    }
    
    pub fn get(&self, field: &str) -> Option<&String> {
        self.fields.get(field)
    }
}

/// Bibliography collection
#[derive(Default)]
pub struct Bibliography {
    pub entries: HashMap<String, BibEntry>
}

impl fmt::Debug for Bibliography {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bibliography")
            .field("entries_count", &self.entries.len())
            .field("entries", &self.entries)
            .finish()
    }
}

impl Bibliography {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new()
        }
    }
    
    pub fn insert(&mut self, entry: BibEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }
    
    pub fn get(&self, key: &str) -> Option<&BibEntry> {
        self.entries.get(key)
    }
    
    pub fn iter(&self) -> impl Iterator<Item = &BibEntry> {
        self.entries.values()
    }
    
    /// Parse a BBL file into Bibliography structure
    pub fn parse_bbl(content: &str) -> Result<Self, BibExtractError> {
        let mut bibliography = Self::new();
        if !content.contains("\\begin{thebibliography}") {
            return Ok(bibliography);
        }

        let bib_content = content
            .split("\\begin{thebibliography}")
            .nth(1)
            .unwrap_or("")
            .split("\\end{thebibliography}")
            .next()
            .unwrap_or("");
 
        let bibitem_re = regex::Regex::new(r"\\bibitem(?:\[[^\]]*\])?\{([^}]+)\}").unwrap();

        // Collect positions of all \bibitem occurrences
        let mut positions = Vec::new();
        for m in bibitem_re.find_iter(bib_content) {
            positions.push(m.start());
        }

        // Extract each bibitem block by slicing from start of one to start of next
        for (i, &start) in positions.iter().enumerate() {
            let end = positions.get(i + 1).copied().unwrap_or_else(|| bib_content.len());
            let item = &bib_content[start..end];

            // Extract key from the bibitem
            let key = bibitem_re.captures(item)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str())
                .unwrap_or("unknown");

            let mut entry_builder = BibEntry::builder(key.to_string(), "article");

            let lines: Vec<&str> = item.trim().lines().collect();
            if lines.len() > 1 {

                // et al handling
                let author_line = lines[1].trim();
                if author_line.contains("et~al.") || author_line.contains("et al.") {
                    // If "et al." is present, we take the string until "et al."
                    let et_al_index = author_line.find("et al.").or_else(|| author_line.find("et~al.")).unwrap_or(author_line.len());
                    let author = &author_line[..et_al_index].trim();
                    entry_builder = entry_builder.field("author", author.to_string());
                } else {
                    // Otherwise, we take the full author line
                    entry_builder = entry_builder.field("author", author_line.to_string());
                }
            }

            let year_re = regex::Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap();
            if let Some(cap) = year_re.captures(item) {
                entry_builder = entry_builder.field("year", cap.get(0).map_or("", |m| m.as_str()).to_string());
            }

            // if newblock is present, split the item into blocks
            let blocks: Vec<&str> = item.split("\\newblock").map(str::trim).filter(|s| !s.is_empty()).collect();
            if blocks.len() > 1 {
                // blocks[1] is usually the title (blocks[0] is author line)
                let raw_title = blocks[1].replace('\n', " ");
                let clean_title = raw_title.split_whitespace().collect::<Vec<_>>().join(" ");
                let clean_title = clean_title.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
                entry_builder = entry_builder.field("title", clean_title);
            }
            else {
                // if no newblock, try to extract title from quotes
                // get title from quotations if no newblock                
                // Use regex to find quoted title
                let title_re = regex::Regex::new(r#"(?:``|\"|\')([^\`\"']+)(?:\'\'|\"|\')"#).unwrap();
                if let Some(title_cap) = title_re.captures(item) {
                    let title = title_cap.get(1).map_or("", |m| m.as_str());
                    let clean_title = title.split_whitespace().collect::<Vec<_>>().join(" ");
                    // strip leading/trailing whitespace and punctuation
                    let clean_title = clean_title.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
                    entry_builder = entry_builder.field("title", clean_title);
                }
            }

            // add item as raw field so that we can parse for arXiv IDs or DOI later
            entry_builder = entry_builder.field("raw", item.trim().to_string());

            bibliography.insert(entry_builder.build());
        }
        
        Ok(bibliography)
    }
    
    /// Parse all bibliography files from a list and consolidate them
    pub fn parse_bibliography_files(bbl_files: &[PathBuf]) -> Result<Self, BibExtractError> {
        let mut consolidated_biblio = Self::new();

        // Parse bibliography files if they exist
        for bbl_file in bbl_files {
            if bbl_file.exists() {
                let content = fs::read_to_string(bbl_file).map_err(|e| BibExtractError::IoError(e))?;
                // Using custom BBL parser
                match Self::parse_bbl(&content) {
                    Ok(bib) => {
                        // Add all entries to our consolidated bibliography
                        for entry in bib.iter() {
                            consolidated_biblio.insert(entry.clone());
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to parse BBL file {:?}: {}", bbl_file, e);
                    }
                }
            }
        }

        Ok(consolidated_biblio)
    }
    
    /// Normalize a citation key based on BibEntry data
    pub fn normalize_citation_key(&self, entry: &BibEntry) -> String {
        // Get the author's last name (first author if multiple)
        let author = entry.get("author")
            .map(|authors| {
                // Extract the first author
                let first_author = if authors.contains(",") {
                    authors.split(",").next().unwrap_or(authors)
                } else if authors.contains(" and ") {
                    authors.split(" and ").next().unwrap_or(authors)
                } else {
                    authors
                };
                
                // Remove "et al." if present
                let first_author = first_author.split("et al")
                    .next()
                    .unwrap_or(first_author)
                    .trim();
                
                // Clean and extract just the last name
                let clean_first_author = clean_text(first_author);
                let words: Vec<&str> = clean_first_author.split('_').collect();
                
                // Return the last word (likely the last name) or the whole name if only one word
                if words.len() > 1 {
                    words.last().unwrap_or(&"unknown").to_string()
                } else {
                    clean_first_author
                }
            })
            .unwrap_or_else(|| "unknown".to_string());
        
        // Get the year
        let year = entry.get("year")
            .map(|y| clean_text(y))
            .unwrap_or_else(String::new);
        
        // Get significant words from title
        let title_words = entry.get("title")
            .map(|title| {
                let clean_title = clean_text(title);
                
                clean_title.split('_')
                    .filter(|w| w.len() > 3)  // Only keep significant words
                    .take(3)                  // Take at most 3 words
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Build the normalized key: lastname_word1_word2_word3_year
        let mut key_parts = vec![author];
        
        // Add title words
        key_parts.extend(title_words);
        
        // Add year at the end if available
        if !year.is_empty() {
            key_parts.push(year);
        }
        
        // Join all parts with underscore
        key_parts.join("_")
    }
    
    /// Extract arXiv ID from a paper title or entry fields
    pub fn extract_arxiv_id(&self, entry: &BibEntry) -> Option<String> {
        // Check if the title or journal field contains "arXiv" followed by an ID pattern
        let fields_to_check = ["title", "journal", "note", "raw"];
        
        for field in fields_to_check {
            if let Some(content) = entry.get(field) {
                // Look for the standard arXiv ID pattern
                if let Some(captures) = ARXIV_ID_REGEX.captures(content) {
                    if let Some(id_match) = captures.get(1) {
                        return Some(id_match.as_str().to_string());
                    }
                }
            }
        }
        
        // Check if the key itself looks like an arXiv ID
        if let Some(captures) = ARXIV_KEY_REGEX.captures(&entry.key) {
            if let Some(id_match) = captures.get(1) {
                return Some(id_match.as_str().to_string());
            }
        }
        
        None
    }
    
    /// Get BibTeX entry from arXiv for a given arXiv ID
    pub fn get_arxiv_bibtex(&self, arxiv_id: &str) -> Result<Option<String>, BibExtractError> {
        let client = Client::new();
        let url = format!("https://arxiv.org/bibtex/{}", arxiv_id);
        
        info!("Fetching BibTeX from arXiv for ID: {}", arxiv_id);
        let response = client.get(&url).send().map_err(|e| BibExtractError::NetworkError(e))?;
        
        if !response.status().is_success() {
            log::warn!("arXiv BibTeX service returned status {}", response.status());
            return Ok(None);
        }
        
        let content = response.text().map_err(|e| BibExtractError::NetworkError(e))?;
        if content.contains("@") && content.contains("author") && content.contains("title") {
            return Ok(Some(content));
        }
        
        Ok(None)
    }
    
    /// Normalize citation keys in LaTeX content
    pub fn normalize_citations(
        &self,
        content: &str
    ) -> Result<(String, HashMap<String, String>), BibExtractError> {
        let mut normalized_content = content.to_string();
        let mut key_map: HashMap<String, String> = HashMap::new();
        
        // Find all citations
        for cap in CITE_REGEX.captures_iter(content) {
            let full_citation = cap.get(0).unwrap().as_str();
            let cite_command = full_citation.split('{').next().unwrap_or("");
            let cite_keys_str = cap.get(1).unwrap().as_str();
            let cite_keys: Vec<&str> = cite_keys_str.split(',').map(|s| s.trim()).collect();
            
            let mut normalized_keys = Vec::new();
            
            for &key in &cite_keys {
                if let Some(entry) = self.get(key) {
                    let normalized_key = self.normalize_citation_key(entry);
                    key_map.insert(key.to_string(), normalized_key.clone());
                    normalized_keys.push(normalized_key);
                } else {
                    // Keep original key if not found in bibliography
                    normalized_keys.push(key.to_string());
                }
            }
            
            // Create the new citation command with proper escaping for curly braces in format string
            let new_citation = format!("{}{{{}}}", cite_command, normalized_keys.join(", "));
            
            // Replace in the content
            normalized_content = normalized_content.replace(full_citation, &new_citation);
        }
        
        Ok((normalized_content, key_map))
    }
}