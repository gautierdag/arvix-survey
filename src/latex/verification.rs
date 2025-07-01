use crate::error::BibExtractError;
use anyhow::Result;
use log::info;
use reqwest::blocking::Client;
use serde_json::Value;
use once_cell::sync::Lazy;
use backoff::retry;
use backoff::ExponentialBackoff;

use crate::latex::{Bibliography, BibEntry, BibEntryBuilder, BIBTEX_ENTRY_REGEX, BIBTEX_FIELD_REGEX};

// Use a single, lazily-initialized reqwest::Client for all API calls to enable connection pooling.
static HTTP_CLIENT: Lazy<Client> = Lazy::new(Client::new);

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
            if let Some(field) = cap.get(1) {
                // The value can be in the second or third capture group depending on whether it's in braces or quotes
                if let Some(value) = cap.get(2).or_else(|| cap.get(3)) {
                    builder = builder.field(field.as_str(), value.as_str().to_string());
                }
            }
        }
        Some(builder.build())
    }

    /// Query DBLP API for paper information based on paper title and author
    pub fn query_dblp_api(&self, entry: &BibEntry) -> Result<Option<Value>, BibExtractError> {
        let title = match entry.get("title") {
            Some(t) => t,
            None => return Ok(None), // No title, can't search
        };
        
        let clean_title = title.replace("{", "").replace("}", "");
        let encoded_title = clean_title.replace(" ", "+");
        let url = format!("https://dblp.org/search/publ/api?q={}&format=json", encoded_title);
        
        let operation = || {
            info!("Querying DBLP API for paper: {}", clean_title);
            let response = HTTP_CLIENT.get(&url).send()
                .map_err(|e| backoff::Error::transient(BibExtractError::NetworkError(e)))?;

            if response.status().is_success() {
                response.json::<Value>().map_err(|e| backoff::Error::transient(BibExtractError::NetworkError(e)))
            } else {
                log::warn!("DBLP API returned status {}", response.status());
                Err(backoff::Error::transient(BibExtractError::ApiError(format!("DBLP API returned status {}", response.status()))))
            }
        };

        match retry(ExponentialBackoff::default(), operation) {
            Ok(json_response) => {
                if let Some(hit_count) = json_response
                    .get("result")
                    .and_then(|r| r.get("hits"))
                    .and_then(|h| h.get("@total"))
                    .and_then(|t| t.as_str())
                    .and_then(|s| s.parse::<i32>().ok())
                {
                    if hit_count > 0 {
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
                }
                Ok(None)
            },
            Err(_) => Ok(None)
        }
    }
    
    /// Find the best matching entry in DBLP results for a given entry
    pub fn find_best_match_in_dblp(&self, dblp_results: &Value, entry: &BibEntry) -> Option<Value> {
        /*
        This function finds the best matching entry in DBLP results based on title, year, and author.
        It compares the original entry's title, year, and author with the titles, years,
        and authors in the DBLP results, scoring matches based on exact matches, year matches,
        and author matches. The best match is returned if the score is above a certain threshold.
        If no suitable match is found, it returns None.
        
        The scoring system is as follows:
        - Exact title match: 3 points
        - Title contains hit title or vice versa: 2 points
        - Year match: 1 point
        - Author match (5 or more matching words): 2 points
        
        The best match is the one with the highest score.
        If no match has a score of at least 2, None is returned.
        */

        let hits = dblp_results
            .get("result")
            .and_then(|r| r.get("hits"))
            .and_then(|h| h.get("hit"))
            .and_then(|h| h.as_array())?;
        
        let original_title = entry.get("title")?;
        let original_year = entry.get("year")?;
        let original_author = entry.get("author")?;

        let mut best_match = None;
        let mut best_score = 0;
        
        for hit in hits {
            let info = hit.get("info")?;
            let hit_title = info.get("title").and_then(|t| t.as_str())?;
            let hit_year = info.get("year").and_then(|y| y.as_str())?;

            let mut score = 0;
            if hit_year == original_year {
                score += 1;
            }
            let clean_original = original_title.to_lowercase().replace("{", "").replace("}", "");
            let clean_hit = hit_title.to_lowercase();
            
            if clean_original == clean_hit {
                score += 3;
            } else if clean_original.contains(&clean_hit) || clean_hit.contains(&clean_original) {
                score += 2;
            }
            else {
                let original_words: Vec<&str> = clean_original.split_whitespace().collect();
                let hit_words: Vec<&str> = clean_hit.split_whitespace().collect();
                
                let matching_words = original_words.iter()
                    .filter(|&word| hit_words.contains(word))
                    .count();
                
                if matching_words > 2 {
                    score += 1;
                }
            }
            // Author matching
            if let Some(original_author) = Some(original_author) {
                if let Some(hit_authors) = info.get("authors").and_then(|a| a.get("author")).and_then(|a| a.as_array()) {
                    let hit_author_names: Vec<String> = hit_authors.iter()
                        .filter_map(|a| a.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
                        .collect();
                    
                    if !hit_author_names.is_empty() {
                        let hit_authors_text = hit_author_names.join(" and ");
                        
                        // Split on both spaces and tildes for word extraction
                        let original_author_words: Vec<&str> = original_author
                            .split(|c| c == ' ' || c == '~')
                            .filter(|w| !w.is_empty())
                            .collect();
                        let hit_author_words: Vec<&str> = hit_authors_text
                            .split(|c| c == ' ' || c == '~')
                            .filter(|w| !w.is_empty())
                            .collect();
                        
                        let matching_author_words = original_author_words.iter()
                            .filter(|&word| hit_author_words.contains(word))
                            .count();
                        
                        if matching_author_words >= 5 {
                            score += 2;
                        }
                    }
                }
            }

            if score > best_score {
                best_score = score;
                best_match = Some(info.clone());
            }
        }

         

        if best_score >= 2 {
            best_match
        } else {
            None
        }
    }

    /// Verifies a BibEntry using the arXiv API.
    fn verify_from_arxiv(&self, entry: &BibEntry) -> Result<Option<BibEntry>, BibExtractError> {
        if let Some(arxiv_id) = self.extract_arxiv_id(entry) {
            let bibtex = self.get_arxiv_bibtex(&arxiv_id)?;
            if let Some(bibtex) = bibtex {
                if let Some(mut verified_entry) = self.parse_bibtex_entry(&bibtex) {
                    verified_entry.set("verified_source", "arXiv".to_string());
                    return Ok(Some(verified_entry));
                }
            }
        }
        Ok(None)
    }

    /// Verifies a BibEntry using the DBLP API.
    fn verify_from_dblp(&self, entry: &BibEntry) -> Result<Option<BibEntry>, BibExtractError> {
        if let Some(dblp_results) = self.query_dblp_api(entry)? {
            if let Some(best_match) = self.find_best_match_in_dblp(&dblp_results, entry) {
                let mut builder = BibEntryBuilder::new(entry.key.clone(), entry.entry_type.clone());

                for (field, value) in &entry.fields {
                    if field != "verified_source" {
                        builder = builder.field(field, value);
                    }
                }

                if let Some(title) = best_match.get("title").and_then(|t| t.as_str()) {
                    builder = builder.field("title", title);
                }

                if let Some(year) = best_match.get("year").and_then(|y| y.as_str()) {
                    builder = builder.field("year", year);
                }

                if let Some(venue) = best_match.get("venue").and_then(|v| v.as_str()) {
                    builder = builder.field("booktitle", venue);
                }

                if let Some(url) = best_match.get("url").and_then(|u| u.as_str()) {
                    builder = builder.field("url", url);
                }

                if let Some(volume) = best_match.get("volume").and_then(|v| v.as_str()) {
                    builder = builder.field("volume", volume);
                }

                if let Some(doi) = best_match.get("doi").and_then(|d| d.as_str()) {
                    builder = builder.field("doi", doi);
                }

                if let Some(authors) = best_match.get("authors").and_then(|a| a.get("author")).and_then(|a| a.as_array()) {
                    let author_names: Vec<String> = authors.iter()
                        .filter_map(|a| a.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
                        .collect();

                    if !author_names.is_empty() {
                        let cleaned_authors: Vec<String> = author_names.iter()
                            .map(|name| {
                                let parts: Vec<&str> = name.split_whitespace().collect();
                                if parts.len() > 1 && parts.last().unwrap().chars().all(char::is_numeric) {
                                    parts[..parts.len() - 1].join(" ")
                                } else {
                                    name.clone()
                                }
                            })
                            .collect();
                        builder = builder.field("author", cleaned_authors.join(" and "));
                    }
                }

                builder = builder.field("verified_source", "DBLP");

                return Ok(Some(builder.build()));
            }
        }
        Ok(None)
    }

    /// Updates a BibEntry with verified data, prioritizing arXiv.
    fn update_entry_with_verified_data(&self, entry: &mut BibEntry, arxiv_result: Option<BibEntry>, dblp_result: Option<BibEntry>) -> bool {
        let (source, verified_entry) = match (arxiv_result, dblp_result) {
            (Some(arxiv_entry), _) => ("arXiv", arxiv_entry),
            (_, Some(dblp_entry)) => ("DBLP", dblp_entry),
            _ => return false,
        };

        for (field, value) in verified_entry.fields.iter() {
            if field != "raw" {
                entry.set(field, value.clone());
            }
        }
        entry.set("verified_source", source.to_string());
        true
    }

    /// Verify a single entry using both DBLP and arXiv APIs
    pub fn verify_entry(&self, entry: &mut BibEntry) -> Result<bool, BibExtractError> {
        let arxiv_result = self.verify_from_arxiv(entry)?;
        let dblp_result = self.verify_from_dblp(entry)?;
        Ok(self.update_entry_with_verified_data(entry, arxiv_result, dblp_result))
    }
}