use anyhow::Result;
use log::info;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::latex::{clean_text, CITE_REGEX, ARXIV_ID_REGEX, ARXIV_KEY_REGEX};

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
    
    /// Convert bibliography to a formatted string representation
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        
        output.push_str("Bibliography {\n");
        
        // Sort entries by key for consistent output
        let mut keys: Vec<_> = self.entries.keys().collect();
        keys.sort();
        
        for key in keys {
            if let Some(entry) = self.entries.get(key) {

                // normalize citation key
                let normalized_key = self.normalize_citation_key(entry);

                output.push_str(&format!("  {}: {} {{\n", normalized_key, entry.entry_type));
                
                // Sort fields for consistent output
                let mut fields: Vec<_> = entry.fields.keys()
                    .filter(|&k| k != "raw") // Exclude raw field
                    .collect();
                fields.sort();
                
                for field in fields {
                    if let Some(value) = entry.fields.get(field) {
                        output.push_str(&format!("    {}: \"{}\",\n", field, value));
                    }
                }
                
                output.push_str("  }\n");
            }
        }
        
        output.push_str("}\n");
        output
    }

    /// Parse a BBL file into Bibliography structure
    pub fn parse_bbl(content: &str) -> Result<Self> {
        let mut bibliography = Self::new();
        
        // Ensure we have thebibliography environment
        if !content.contains("\\begin{thebibliography}") || !content.contains("\\end{thebibliography}") {
            return Ok(bibliography); // Return empty bibliography if invalid format
        }
        
        // Extract just the content between \begin{thebibliography} and \end{thebibliography}
        let start_idx = content.find("\\begin{thebibliography}").unwrap_or(0);
        let end_idx = content.find("\\end{thebibliography}").unwrap_or(content.len());
        let bib_content = &content[start_idx..end_idx];
        
        // Split on \bibitem to get individual entries
        let bibitem_parts: Vec<&str> = bib_content.split("\\bibitem").skip(1).collect();
        
        // Regex for citation key extraction (looking for the format used in the test BBL content)
        let re_citeauthoryear = regex::Regex::new(r"^\[\\protect\\citeauthoryear\{([^}]+)\}\{(\d{4})\}\]\{([^}]+)\}")?;
        let re_citation_key = regex::Regex::new(r"^\{([^}]+)\}")?; // Simplified pattern for {key}
        
        for part in bibitem_parts {
            let lines: Vec<&str> = part.lines().collect();
            if lines.is_empty() {
                continue;
            }
            
            let first_line = lines[0].trim();
            let mut citation_key = String::new();
            let mut year = String::new();
            
            // Extract citation key and year using the \citeauthoryear pattern
            if let Some(captures) = re_citeauthoryear.captures(first_line) {
                if let Some(year_match) = captures.get(2) {
                    year = year_match.as_str().to_string();
                }
                
                if let Some(key_match) = captures.get(3) {
                    citation_key = key_match.as_str().to_string();
                }
            } 
            // Try simplified bracket pattern {key} if \citeauthoryear doesn't match
            else if let Some(captures) = re_citation_key.captures(first_line) {
                if let Some(key_match) = captures.get(1) {
                    citation_key = key_match.as_str().to_string();
                }
            }
            
            // If we still don't have a key, extract anything in braces at the end of the line
            if citation_key.is_empty() {
                let braces_re = regex::Regex::new(r"\{([^{}]+)\}\s*$")?;
                if let Some(captures) = braces_re.captures(first_line) {
                    if let Some(key_match) = captures.get(1) {
                        citation_key = key_match.as_str().to_string();
                    }
                }
            }
            
            // Skip entry if no key found
            if citation_key.is_empty() {
                continue;
            }
            
            // Create a builder for the entry
            let mut entry_builder = BibEntryBuilder::new(citation_key.clone(), "article".to_string());
            
            // Extract author from second line if available
            if lines.len() > 1 {
                entry_builder = entry_builder.field("author", lines[1].trim().to_string());
            }
            
            // Set year from \citeauthoryear or look for year in content
            if !year.is_empty() {
                entry_builder = entry_builder.field("year", year);
            } else {
                // Look for year in the content
                // First look for year at the end followed by a period
                let year_end_re = regex::Regex::new(r"(\d{4})\.$")?;
                let mut found_year = false;
                for line in lines.iter().rev() {
                    if let Some(captures) = year_end_re.captures(line) {
                        if let Some(year_match) = captures.get(1) {
                            entry_builder = entry_builder.field("year", year_match.as_str().to_string());
                            found_year = true;
                            break;
                        }
                    }
                }
                
                // If still no year, look for the first 4-digit number that could be a year
                if !found_year {
                    let year_re = regex::Regex::new(r"\b(19\d{2}|20\d{2})\b")?;
                    for line in &lines {
                        if let Some(captures) = year_re.captures(line) {
                            if let Some(year_match) = captures.get(1) {
                                entry_builder = entry_builder.field("year", year_match.as_str().to_string());
                                break;
                            }
                        }
                    }
                }
            }
            
            // Try to extract title - usually starts with \newblock
            let title_re = regex::Regex::new(r"\\newblock\s+(.*?)(?:\.|\n)")?;
            let full_text = lines.join("\n");
            if let Some(captures) = title_re.captures(&full_text) {
                if let Some(title_match) = captures.get(1) {
                    entry_builder = entry_builder.field("title", title_match.as_str().trim().to_string());
                }
            }
            
            // Store the raw content for debugging
            entry_builder = entry_builder.field("raw", part.trim().to_string());
            
            // Build the entry and add it to the bibliography
            bibliography.insert(entry_builder.build());
        }
        
        Ok(bibliography)
    }
    
    /// Parse all bibliography files from a list and consolidate them
    pub fn parse_bibliography_files(bbl_files: &[PathBuf]) -> Result<Self> {
        let mut consolidated_biblio = Self::new();

        // Parse bibliography files if they exist
        for bbl_file in bbl_files {
            if bbl_file.exists() {
                let content = fs::read_to_string(bbl_file)?;
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
    pub fn get_arxiv_bibtex(&self, arxiv_id: &str) -> Result<Option<String>> {
        let client = Client::new();
        let url = format!("https://arxiv.org/bibtex/{}", arxiv_id);
        
        info!("Fetching BibTeX from arXiv for ID: {}", arxiv_id);
        let response = client.get(&url).send()?;
        
        if !response.status().is_success() {
            log::warn!("arXiv BibTeX service returned status {}", response.status());
            return Ok(None);
        }
        
        let content = response.text()?;
        if content.contains("@") && content.contains("author") && content.contains("title") {
            return Ok(Some(content));
        }
        
        Ok(None)
    }
    
    /// Normalize citation keys in LaTeX content
    pub fn normalize_citations(
        &self,
        content: &str
    ) -> Result<(String, HashMap<String, String>)> {
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