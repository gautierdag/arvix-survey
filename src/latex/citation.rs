use anyhow::Result;
use crate::error::BibExtractError;
use regex::Regex;

use std::collections::HashMap;

use crate::latex::{Bibliography, CITE_REGEX};

/// Structure representing an extracted section from a LaTeX document
#[derive(Debug)]
pub struct ExtractedSection {
    pub title: String,                         // The title of the section
    pub content: String,                       // The content of the section (raw LaTeX)
    pub citations: Vec<String>,                // List of citations found in the section
}

/// Check if a section title indicates a related work section
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
pub fn extract_sections_from_latex(content: &str, _bibliography: &Bibliography) -> Result<Vec<ExtractedSection>, BibExtractError> {
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
fn extract_title_from_section(section_text: &str) -> Result<String, BibExtractError> {
    // Use regex to extract title from the beginning of the text
    let title_re = Regex::new(r"^\s*\{([^}]*)\}").map_err(|e| BibExtractError::ApiError(e.to_string()))?;
    
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
fn extract_content_from_section(section_text: &str) -> Result<String, BibExtractError> {
    // Extract content after the title
    let title_re = Regex::new(r"^\s*\{[^}]*\}").map_err(|e| BibExtractError::ApiError(e.to_string()))?;
    
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

/// Normalize citations in LaTeX content
pub fn normalize_citations(
    bibliography: &Bibliography,
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
            if let Some(entry) = bibliography.get(key) {
                let normalized_key = bibliography.normalize_citation_key(entry);
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