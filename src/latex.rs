use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::fs;
use log::info;
use reqwest::blocking::Client;
use std::io::{self, Read, Write, Seek, SeekFrom};
use tempfile::TempDir;
use zip::ZipArchive;
use flate2::read::GzDecoder;
use tar::Archive;
use std::collections::HashMap;
use walkdir::WalkDir;
use std::fmt;
use serde_json::Value;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

// Commonly used regex patterns compiled once
static CITE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\\(?:cite|citep|citet|citealp|citeauthor)\{([^}]+)\}").expect("Invalid citation regex pattern")
});
static ARXIV_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"arXiv:?\s*([0-9]+\.[0-9]+)").expect("Invalid arXiv ID regex pattern")
});
static ARXIV_KEY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([0-9]{4}\.[0-9]+)$").expect("Invalid arXiv key regex pattern")
});
static BIBTEX_ENTRY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"@([a-zA-Z]+)\{([^,]+),").expect("Invalid BibTeX entry regex pattern")
});
static BIBTEX_FIELD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z]+)\s*=\s*\{([^{}]*((\{[^{}]*\})[^{}]*)*)\}").expect("Invalid BibTeX field regex pattern")
});

/// Helper function to clean text by removing punctuation and special characters
fn clean_text(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("_")
        .to_lowercase()
}

/// Custom bibliography entry structure
#[derive(Debug, Clone)]
pub struct BibEntry {
    pub key: String,
    pub entry_type: String,
    pub fields: HashMap<String, String>,
}

impl BibEntry {
    pub fn new(key: String, entry_type: String) -> Self {
        Self {
            key,
            entry_type,
            fields: HashMap::new(),
        }
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
    entries: HashMap<String, BibEntry>
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
                let normalized_key =self.normalize_citation_key(entry);

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
        let re_citeauthoryear = Regex::new(r"^\[\\protect\\citeauthoryear\{([^}]+)\}\{(\d{4})\}\]\{([^}]+)\}")?;
        let re_citation_key = Regex::new(r"^\{([^}]+)\}")?; // Simplified pattern for {key}
        
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
                let braces_re = Regex::new(r"\{([^{}]+)\}\s*$")?;
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
            
            // Create and populate the entry
            let mut entry = BibEntry::new(citation_key.clone(), "article".to_string());
            
            // Extract author from second line if available
            if lines.len() > 1 {
                entry.set("author", lines[1].trim().to_string());
            }
            
            // Set year from \citeauthoryear or look for year in content
            if !year.is_empty() {
                entry.set("year", year);
            } else {
                // Look for year in the content
                // First look for year at the end followed by a period
                let year_end_re = Regex::new(r"(\d{4})\.$")?;
                for line in lines.iter().rev() {
                    if let Some(captures) = year_end_re.captures(line) {
                        if let Some(year_match) = captures.get(1) {
                            entry.set("year", year_match.as_str().to_string());
                            break;
                        }
                    }
                }
                
                // If still no year, look for the first 4-digit number that could be a year
                if entry.get("year").is_none() {
                    let year_re = Regex::new(r"\b(19\d{2}|20\d{2})\b")?;
                    for line in &lines {
                        if let Some(captures) = year_re.captures(line) {
                            if let Some(year_match) = captures.get(1) {
                                entry.set("year", year_match.as_str().to_string());
                                break;
                            }
                        }
                    }
                }
            }
            
            // Try to extract title - usually starts with \newblock
            let title_re = Regex::new(r"\\newblock\s+(.*?)(?:\.|\n)")?;
            let full_text = lines.join("\n");
            if let Some(captures) = title_re.captures(&full_text) {
                if let Some(title_match) = captures.get(1) {
                    entry.set("title", title_match.as_str().trim().to_string());
                }
            }
            
            // Store the raw content for debugging
            entry.set("raw", part.trim().to_string());
            
            // Add the entry to the bibliography
            bibliography.insert(entry);
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
    
    /// Query DBLP API for paper information based on paper title and author
    pub fn query_dblp_api(&self, entry: &BibEntry) -> Result<Option<Value>> {
        let client = Client::new();
        
        // Get paper title and clean it for search
        let title = match entry.get("title") {
            Some(t) => t,
            None => return Ok(None), // No title, can't search
        };
        
        // Clean the title a bit for better search
        let clean_title = title.replace("{", "").replace("}", "");
        
        // URL encode the title for the query
        let encoded_title = clean_title.replace(" ", "+");
        let url = format!("https://dblp.org/search/publ/api?q={}&format=json", encoded_title);
        
        info!("Querying DBLP API for paper: {}", clean_title);
        let response = client.get(&url).send()?;
        if !response.status().is_success() {
            log::warn!("DBLP API returned status {}", response.status());
            return Ok(None);
        }
        
        let json_response: Value = response.json()?;
        
        // Check if we have any results
        if let Some(hit_count) = json_response
            .get("result")
            .and_then(|r| r.get("hits"))
            .and_then(|h| h.get("@total"))
            .and_then(|t| t.as_str())
            .and_then(|s| s.parse::<i32>().ok())
        {
            if hit_count == 0 {
                return Ok(None);
            }
            // Get the hits array
            if let Some(hits) = json_response
                .get("result")
                .and_then(|r| r.get("hits"))
                .and_then(|h| h.get("hit"))
                .and_then(|h| h.as_array())
            {
                if !hits.is_empty() {
                    return Ok(Some(json_response));
                }
            }
        }
        
        Ok(None)
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
    
    /// Parse a BibTeX entry string into a BibEntry
    fn parse_bibtex_entry(&self, bibtex: &str) -> Option<BibEntry> {
        // Simple BibTeX parser for our needs
        // Extract entry type and key 
        let (entry_type, entry_key) = BIBTEX_ENTRY_REGEX.captures(bibtex).and_then(|caps| {
            let etype = caps.get(1).map(|m| m.as_str().to_string())?;
            let ekey = caps.get(2).map(|m| m.as_str().to_string())?;
            Some((etype, ekey))
        })?;
        
        let mut entry = BibEntry::new(entry_key, entry_type);
        
        // Extract fields
        for cap in BIBTEX_FIELD_REGEX.captures_iter(bibtex) {
            if let (Some(field), Some(value)) = (cap.get(1), cap.get(2)) {
                entry.set(field.as_str(), value.as_str().to_string());
            }
        }
        
        Some(entry)
    }
    
    /// Find the best matching entry in DBLP results for a given entry
    fn find_best_match_in_dblp(&self, dblp_results: &Value, entry: &BibEntry) -> Option<Value> {
        let hits = dblp_results
            .get("result")
            .and_then(|r| r.get("hits"))
            .and_then(|h| h.get("hit"))
            .and_then(|h| h.as_array())?;
        
        let original_title = entry.get("title")?;
        let original_year = entry.get("year")?;
        
        let mut best_match = None;
        let mut best_score = 0;
        
        for hit in hits {
            let info = hit.get("info")?;
            // Extract title and year for comparison
            let hit_title = info.get("title").and_then(|t| t.as_str())?;
            let hit_year = info.get("year").and_then(|y| y.as_str())?;
            // Simple scoring: +1 for matching year, +3 for having similar title
            let mut score = 0;
            // Year exact match
            if hit_year == original_year {
                score += 1;
            }
            // Title similarity (very basic - could be improved)
            let clean_original = original_title.to_lowercase().replace("{", "").replace("}", "");
            let clean_hit = hit_title.to_lowercase();
            
            if clean_original == clean_hit {
                score += 3;
            } else if clean_original.contains(&clean_hit) || clean_hit.contains(&clean_original) {
                score += 2;
            } else {
                // Count matching words
                let original_words: Vec<&str> = clean_original.split_whitespace().collect();
                let hit_words: Vec<&str> = clean_hit.split_whitespace().collect();
                
                let matching_words = original_words.iter()
                    .filter(|&word| hit_words.contains(word))
                    .count();
                
                if matching_words > 2 {
                    score += 1;
                }
            }
            if score > best_score {
                best_score = score;
                best_match = Some(info.clone());
            }
        }
        // Only return match if score is reasonable
        if best_score >= 2 {
            best_match
        } else {
            None
        }
    }
    
    
    /// Verify a single entry using both DBLP and arXiv APIs in parallel
    pub fn verify_entry(&self, entry: &mut BibEntry) -> Result<bool> {
        use rayon::prelude::*;
        
        // Define a function to verify entry from arXiv
        let verify_from_arxiv = |entry: &BibEntry| -> Option<(String, BibEntry)> {
            if let Some(arxiv_id) = self.extract_arxiv_id(entry) {
                let temp_bib = Bibliography::new();
                match temp_bib.get_arxiv_bibtex(&arxiv_id) {
                    Ok(Some(bibtex)) => {
                        if let Some(verified_entry) = temp_bib.parse_bibtex_entry(&bibtex) {
                            return Some(("arXiv".to_string(), verified_entry));
                        }
                    },
                    _ => {}
                }
            }
            None
        };
        
        // Define a function to verify entry from DBLP
        let verify_from_dblp = |entry: &BibEntry| -> Option<(String, BibEntry)> {
            let temp_bib = Bibliography::new();
            match temp_bib.query_dblp_api(entry) {
                Ok(Some(dblp_results)) => {
                    if let Some(best_match) = temp_bib.find_best_match_in_dblp(&dblp_results, entry) {
                        let mut dblp_entry = entry.clone();
                        
                        // Update fields from DBLP result
                        if let Some(title) = best_match.get("title").and_then(|t| t.as_str()) {
                            dblp_entry.set("title", title.to_string());
                        }
                        
                        if let Some(year) = best_match.get("year").and_then(|y| y.as_str()) {
                            dblp_entry.set("year", year.to_string());
                        }
                        
                        if let Some(venue) = best_match.get("venue").and_then(|v| v.as_str()) {
                            dblp_entry.set("booktitle", venue.to_string());
                        }

                        // url 
                        if let Some(url) = best_match.get("url").and_then(|u| u.as_str()) {
                            dblp_entry.set("url", url.to_string());
                        }
                        // volume
                        if let Some(volume) = best_match.get("volume").and_then(|v| v.as_str()) {
                            dblp_entry.set("volume", volume.to_string());
                        }
                        // doi
                        if let Some(doi) = best_match.get("doi").and_then(|d| d.as_str()) {
                            dblp_entry.set("doi", doi.to_string());
                        }
                        
                        if let Some(authors) = best_match.get("authors")
                            .and_then(|a| a.get("author"))
                            .and_then(|a| a.as_array()) 
                        {
                            let author_names: Vec<String> = authors.iter()
                                .filter_map(|a| a.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
                                .collect();
                            
                            if !author_names.is_empty() {
                                // clean author names that have numbers after their names (e.g. "John Doe 001" -> "John Doe")
                                let cleaned_authors: Vec<String> = author_names.iter()
                                    .map(|name| {
                                        let parts: Vec<&str> = name.split_whitespace().collect();
                                        if parts.len() > 1 && parts.last().unwrap().chars().all(char::is_numeric) {
                                            parts[..parts.len()-1].join(" ")
                                        } else {
                                            name.clone()
                                        }
                                    })
                                    .collect();
                                dblp_entry.set("author", cleaned_authors.join(" and "));
                            }
                        }
                        
                        dblp_entry.set("verified_source", "DBLP".to_string());
                        return Some(("DBLP".to_string(), dblp_entry));
                    }
                },
                _ => {}
            }
            None
        };
        
        // Create a vector of verification functions
        let entry_clone = entry.clone();
        let result = [
            verify_from_arxiv(&entry_clone),
            verify_from_dblp(&entry_clone),
        ]
        .into_par_iter()
        .filter_map(|result| result)
        .collect::<Vec<(String, BibEntry)>>();
        
        if !result.is_empty() {
            // Prioritize arXiv over DBLP if we have both
            let (source, verified_entry) = result.iter()
                .find(|(s, _)| s == "arXiv")
                .unwrap_or(&result[0]);
            
            // Update the entry with verified information
            for (field, value) in verified_entry.fields.iter() {
                if field != "raw" {
                    entry.set(field, value.clone());
                }
            }
            entry.set("verified_source", source.clone());
            
            return Ok(true);
        }
        
        Ok(false)
    }
    
}

pub fn related_work_section(section_title: &str) -> bool {
    let related_work_sections = [
        "related work",
        "background",
        "literature review",
        "prior work",
        "previous work",
        "state of the art",
        "comparative analysis",
        "context",
        "existing work",
        "existing approaches",
        "existing methods",
        "review of the literature",        
        "previous approaches",
        "foundation",
    ];

    // Check if the section title matches any of the related work sections
    for section in related_work_sections.iter() {
        if section_title.to_lowercase().contains(section) {
            return true;
        }
    }
    return false;
}


/// Extract sections from LaTeX content
pub fn extract_sections_from_latex(content: &str, _bibliography: &Bibliography) -> Result<Vec<ExtractedSection>> {
    let mut sections = Vec::new();
    
    // Helper function to extract citations from text
    let extract_citations = |text: &str| -> Vec<String> {
        let mut citations = Vec::new();
        for cite_cap in CITE_REGEX.captures_iter(text) {
            let cite_keys = cite_cap.get(1).map_or("", |m| m.as_str());
            for key in cite_keys.split(',') {
                citations.push(key.trim().to_string());
            }
        }
        // Remove duplicates
        citations.sort();
        citations.dedup();
        citations
    };
    
    // Process sections
    let section_parts: Vec<&str> = content.split("\\section").skip(1).collect();
    for section_text in section_parts {
        // Extract the section title from the part
        let title = extract_title_from_section(section_text)?;
        
        // Skip if not a related work section
        if !related_work_section(&title) {
            continue;
        }
        
        // Extract the content (everything after the title)
        let content = extract_content_from_section(section_text)?;
        
        // Extract citations from this section
        let citations = extract_citations(&content);
        
        sections.push(ExtractedSection {
            title,
            content,
            citations,
        });
    }
    
    // Also extract relevant subsections
    let subsection_parts: Vec<&str> = content.split("\\subsection").skip(1).collect();
    for section_text in subsection_parts {
        // Extract the section title from the part
        let title = extract_title_from_section(section_text)?;
        
        // Skip if not a related work section
        if !related_work_section(&title) {
            continue;
        }
        
        // Extract the content (everything after the title)
        let content = extract_content_from_section(section_text)?;
        
        // Extract citations from this section
        let citations = extract_citations(&content);
        
        sections.push(ExtractedSection {
            title,
            content,
            citations,
        });
    }
    
    Ok(sections)
}

/// Helper function to extract section title from a section text
fn extract_title_from_section(section_text: &str) -> Result<String> {
    // Use regex to extract title from the beginning of the text
    let title_re = Regex::new(r"^\s*\{([^}]*)\}")?;
    
    if let Some(cap) = title_re.captures(section_text) {
        if let Some(title_match) = cap.get(1) {
            return Ok(title_match.as_str().trim().to_string());
        }
    }
    
    // Fallback: try to extract title from the first line
    let first_line = section_text.lines().next().unwrap_or("").trim();
    if first_line.starts_with('{') && first_line.contains('}') {
        let end_idx = first_line.find('}').unwrap_or(first_line.len());
        if end_idx > 1 {
            return Ok(first_line[1..end_idx].trim().to_string());
        }
    }
    
    // Last resort: return empty string
    Ok(String::new())
}

/// Helper function to extract content from a section text (everything after the title)
fn extract_content_from_section(section_text: &str) -> Result<String> {
    // Extract content after the title
    let title_re = Regex::new(r"^\s*\{[^}]*\}")?;
    
    if let Some(title_match) = title_re.find(section_text) {
        let content_start = title_match.end();
        if content_start < section_text.len() {
            return Ok(section_text[content_start..].trim().to_string());
        }
    }
    
    // Fallback: find the first line break and start from there
    if let Some(first_line_end) = section_text.find('\n') {
        if first_line_end < section_text.len() {
            return Ok(section_text[first_line_end + 1..].trim().to_string());
        }
    }
    
    // Last resort: return the whole text, it's better than nothing
    Ok(section_text.trim().to_string())
}

pub fn process_arxiv_paper(paper_id: &str) -> Result<ArxivPaper> {
    // Download and extract paper
    let paper = download_arxiv_source(paper_id)?;
    // Return the paper with extracted sections
    Ok(paper)
}

#[derive(Debug)]
pub struct ExtractedSection {
    pub title: String,                         // The title of the section
    pub content: String,                       // The content of the section (raw LaTeX)
    pub citations: Vec<String>,                // List of citations found in the section
}

/// Structure representing an arXiv paper with its associated files
pub struct ArxivPaper {
    pub id: String,                          // arXiv ID
    pub sections: Vec<ExtractedSection>,     // extracted sections
    pub bibliography: Bibliography,          // parsed bibliography
    _temp_dir: TempDir,                      // Temporary directory (keep alive while the paper is used)
}

impl ArxivPaper {
    /// Verify bibliography entries using parallel processing for both sources (DBLP and arXiv simultaneously)
    pub fn verify_bibliography(&mut self) -> Result<usize> {
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

/// Download and process an arXiv paper
pub fn download_arxiv_source(paper_id: &str) -> Result<ArxivPaper> {
    let client = Client::new();
    let url = format!("https://arxiv.org/e-print/{}", paper_id);

    info!("Downloading source files from arXiv for paper: {}", paper_id);
    let response = client.get(&url).send()?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download source: HTTP {}", response.status());
    }

    // Create temp directory to extract files
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Save the downloaded source to a temporary file
    let mut source_file = tempfile::tempfile()?;
    let content = response.bytes()?;
    
    if content.is_empty() {
        anyhow::bail!("Received empty content from arXiv for paper ID: {}", paper_id);
    }
    
    source_file.write_all(&content)?;
    source_file.seek(std::io::SeekFrom::Start(0))?;
    
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
    let sections = extract_sections_from_latex(&full_content, &bibliography)?;

    Ok(ArxivPaper {
        id: paper_id.to_string(),
        sections,
        bibliography,
        _temp_dir: temp_dir,
    })
}

/// Extract archive (supports ZIP and TAR.GZ)
fn extract_archive<R: Read + io::Seek>(mut archive: R, output_dir: &Path) -> Result<()> {
    // Try to open as ZIP first
    match ZipArchive::new(&mut archive) {
        Ok(mut zip) => {
            info!("Extracting ZIP archive");
            for i in 0..zip.len() {
                let mut file = zip.by_index(i)?;
                let outpath = match file.enclosed_name() {
                    Some(path) => output_dir.join(path),
                    None => continue,
                };

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = fs::File::create(&outpath)?;
                    io::copy(&mut file, &mut outfile)?;
                }
            }
            return Ok(());
        },
        Err(_) => {
            // Rewind the file
            archive.seek(SeekFrom::Start(0))?;
            
            // Try as tar.gz
            info!("Trying to extract as TAR.GZ archive");
            let gz = GzDecoder::new(archive);
            let mut tar = Archive::new(gz);
            tar.unpack(output_dir)?;
            return Ok(());
        }
    }
}

/// Find all BBL files in a directory
pub fn find_bbl_files(dir: &Path) -> Result<Vec<PathBuf>> {
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
pub fn find_main_tex_file(dir: &Path) -> Result<PathBuf> {
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
        if let Ok(content) = fs::read_to_string(file) {
            if content.contains("\\documentclass") {
                return Ok(file.clone());
            }
        }
    }
    
    // If we still haven't found anything, just return the first .tex file
    if !tex_files.is_empty() {
        return Ok(tex_files[0].clone());
    }
    
    anyhow::bail!("No LaTeX main file found in {:?}", dir)
}

/// Extract all LaTeX content from files including handling \input commands
pub fn extract_all_latex_from_files(
    base_dir: &Path,
    main_tex_file: &Path,
) -> Result<(String, Vec<PathBuf>)> {
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
) -> Result<String> {
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
    let content = fs::read_to_string(tex_file)?;
    
    // Look for \input and \include commands
    let mut result = String::new();
    let input_re = Regex::new(r"\\(input|include)\{([^}]+)\}")?;
    
    let mut last_end = 0;
    for cap in input_re.captures_iter(&content) {
        let full_match = cap.get(0).unwrap();
        // Add the content before this match
        result.push_str(&content[last_end..full_match.start()]);
        last_end = full_match.end();
        
        // Extract the filename
        let filename = cap.get(2).unwrap().as_str();
        
        // Resolve the path
        if let Ok(Some(input_path)) = resolve_input_path(base_dir, filename) {
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
pub fn resolve_input_path(base_dir: &Path, filename: &str) -> Result<Option<PathBuf>> {
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