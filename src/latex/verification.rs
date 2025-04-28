use anyhow::Result;
use log::info;
use reqwest::blocking::Client;
use serde_json::Value;

use crate::latex::{Bibliography, BibEntry, BibEntryBuilder, BIBTEX_ENTRY_REGEX, BIBTEX_FIELD_REGEX};

/// Verify a single entry using both DBLP and arXiv APIs
impl Bibliography {
    /// Parse a BibTeX entry string into a BibEntry
    pub fn parse_bibtex_entry(&self, bibtex: &str) -> Option<BibEntry> {
        // Simple BibTeX parser for our needs
        // Extract entry type and key 
        let (entry_type, entry_key) = BIBTEX_ENTRY_REGEX.captures(bibtex).and_then(|caps| {
            let etype = caps.get(1).map(|m| m.as_str().to_string())?;
            let ekey = caps.get(2).map(|m| m.as_str().to_string())?;
            Some((etype, ekey))
        })?;
        
        let mut builder = BibEntryBuilder::new(entry_key, entry_type);
        
        // Extract fields
        for cap in BIBTEX_FIELD_REGEX.captures_iter(bibtex) {
            if let (Some(field), Some(value)) = (cap.get(1), cap.get(2)) {
                builder = builder.field(field.as_str(), value.as_str().to_string());
            }
        }
        
        Some(builder.build())
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
                        // Use the builder pattern for creating the updated entry
                        let mut builder = BibEntryBuilder::new(entry.key.clone(), entry.entry_type.clone());
                        
                        // Copy existing fields
                        for (field, value) in &entry.fields {
                            if field != "verified_source" {
                                builder = builder.field(field, value);
                            }
                        }
                        
                        // Update fields from DBLP result
                        if let Some(title) = best_match.get("title").and_then(|t| t.as_str()) {
                            builder = builder.field("title", title);
                        }
                        
                        if let Some(year) = best_match.get("year").and_then(|y| y.as_str()) {
                            builder = builder.field("year", year);
                        }
                        
                        if let Some(venue) = best_match.get("venue").and_then(|v| v.as_str()) {
                            builder = builder.field("booktitle", venue);
                        }

                        // url 
                        if let Some(url) = best_match.get("url").and_then(|u| u.as_str()) {
                            builder = builder.field("url", url);
                        }
                        // volume
                        if let Some(volume) = best_match.get("volume").and_then(|v| v.as_str()) {
                            builder = builder.field("volume", volume);
                        }
                        // doi
                        if let Some(doi) = best_match.get("doi").and_then(|d| d.as_str()) {
                            builder = builder.field("doi", doi);
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
                                builder = builder.field("author", cleaned_authors.join(" and "));
                            }
                        }
                        
                        builder = builder.field("verified_source", "DBLP");
                        
                        return Some(("DBLP".to_string(), builder.build()));
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