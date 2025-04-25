use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::fs;
use log::info;
use reqwest::blocking::Client;
use std::fs::File;
use std::io::{self, Read, Write, Seek, SeekFrom};
use tempfile::TempDir;
use zip::ZipArchive;
use flate2::read::GzDecoder;
use tar::Archive;
use std::collections::HashMap;
use walkdir::WalkDir;
use std::fmt;

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
        // Helper function to clean text by removing punctuation and special characters
        fn clean_text(text: &str) -> String {
            text.chars()
                .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join("_")
                .to_lowercase()
        }
        
        // Get the author's last name (first author if multiple)
        let author = entry.get("author")
            .map(|authors| {
                // First extract the first author
                let first_author = if authors.contains(",") {
                    authors.split(",").next().unwrap_or(authors)
                } else if authors.contains(" and ") {
                    authors.split(" and ").next().unwrap_or(authors)
                } else{
                    authors
                };
                // print!("First author: {}", first_author);
                // Check for "et al." format in the original string
                let first_author = if first_author.contains("et al") {
                    first_author.split("et al").next().unwrap_or(first_author).trim()
                } else {
                    first_author.trim()
                };
                // Clean the first author string
                let clean_first_author = clean_text(first_author);
                // Extract just the last name
                let words: Vec<&str> = clean_first_author.split('_').collect();
                if words.len() > 1 {
                    // Take the last word as the last name
                    words.last().unwrap_or(&"unknown").to_string()
                } else {
                    // If only one word, use that
                    clean_first_author
                }
            })
            .unwrap_or_else(|| "unknown".to_string());
        
        // Get the year and clean it
        let year = entry.get("year")
            .map(|y| clean_text(y))
            .unwrap_or_else(String::new);
        
        // Get the first three significant words from title
        let title_words = entry.get("title")
            .map(|title| {
                // Clean the title by removing punctuation and special characters
                let clean_title = clean_text(title);
                
                clean_title.split('_')
                    .filter(|w| w.len() > 3)  // Only keep significant words
                    .take(3)                  // Take at most 3 words
                    .map(|s| s.to_string())   // Convert to owned strings
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Format: lastname_word1_word2_word3_year
        let mut result = author;
        
        // Add title words
        for word in title_words {
            result.push('_');
            result.push_str(&word);
        }
        
        // Add year at the end
        if !year.is_empty() {
            result.push('_');
            result.push_str(&year);
        }
        
        result
    }
    
    /// Normalize citation keys in LaTeX content
    pub fn normalize_citations(
        &self,
        content: &str
    ) -> Result<(String, HashMap<String, String>)> {
        let mut normalized_content = content.to_string();
        let mut key_map: HashMap<String, String> = HashMap::new();
        
        // Find all citations
        let cite_re = Regex::new(r"\\(?:cite|citep|citet|citealp|citeauthor)\{([^}]+)\}")?;
        
        for cap in cite_re.captures_iter(content) {
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

// Represents a section extracted from a LaTeX paper
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
    source_file.seek(SeekFrom::Start(0))?;

    // Extract the archive
    extract_archive(source_file, temp_path)?;
    
    // Find the main .tex file
    let main_tex_file = find_main_tex_file(temp_path)?;
    
    // Extract all LaTeX content (use _ to indicate we're intentionally not using included_files)
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
    // Try to open as zip archive first
    let mut buf = Vec::new();
    archive.read_to_end(&mut buf)?;

    if let Ok(mut zip) = ZipArchive::new(io::Cursor::new(&buf)) {
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let outpath = output_dir.join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }
        return Ok(());
    }

    // If not a zip, try as tar.gz
    let gz = GzDecoder::new(io::Cursor::new(buf));
    let mut archive = Archive::new(gz);
    archive.unpack(output_dir)?;

    Ok(())
}

/// Find the main LaTeX file in a directory
pub fn find_main_tex_file(dir: &Path) -> Result<PathBuf> {
    let mut tex_files = Vec::new();
    // Recursively find all .tex files
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "tex") {
            tex_files.push(path.to_path_buf());
        }
    }
    if tex_files.is_empty() {
        anyhow::bail!("No .tex files found in the extracted archive.");
    }
    // Look for files that have \begin{document}
    for tex_file in &tex_files {
        if let Ok(content) = fs::read_to_string(tex_file) {
            if !content.is_empty() && content.contains("\\begin{document}") {
                return Ok(tex_file.clone());
            }
        }
    }
    // Look for files that have \documentclass
    for tex_file in &tex_files {
        if let Ok(content) = fs::read_to_string(tex_file) {
            if !content.is_empty() && content.contains("\\documentclass") {
                return Ok(tex_file.clone());
            }
        }
    }
    // Look for common main file names
    let common_names = ["main.tex", "paper.tex", "article.tex", "manuscript.tex"];
    for name in common_names {
        for tex_file in &tex_files {
            if tex_file.file_name().and_then(|f| f.to_str()) == Some(name) {
                if let Ok(content) = fs::read_to_string(tex_file) {
                    if !content.is_empty() {
                        return Ok(tex_file.clone());
                    }
                }
            }
        }
    }
    // Fallback: Return any non-empty .tex file
    for tex_file in &tex_files {
        if let Ok(content) = fs::read_to_string(tex_file) {
            if !content.is_empty() {
                return Ok(tex_file.clone());
            }
        }
    }
    // Last resort: Just return the first .tex file
    Ok(tex_files[0].clone())
}

/// Extract all LaTeX content from files including handling \input commands
pub fn extract_all_latex_from_files(
    base_dir: &Path,
    main_tex_file: &Path,
) -> Result<(String, Vec<PathBuf>)> {
    let mut included_files = Vec::new();
    let mut processed_files = Vec::new();
    
    // Process the main file
    let content = extract_latex_content(base_dir, main_tex_file, &mut included_files, &mut processed_files)?;
    
    // Filter content to what's between \begin{document} and \end{document}
    let doc_re = Regex::new(r"(?s)\\begin\{document\}(.*?)\\end\{document\}")?;
    let filtered_content = match doc_re.captures(&content) {
        Some(cap) => cap.get(1).map_or_else(|| content.clone(), |m| m.as_str().to_string()),
        None => content,
    };
    
    Ok((filtered_content, included_files))
}

/// Recursive helper function to extract LaTeX content
fn extract_latex_content(
    base_dir: &Path,
    tex_file: &Path,
    included_files: &mut Vec<PathBuf>,
    processed_files: &mut Vec<PathBuf>,
) -> Result<String> {
    // Convert Path to PathBuf for comparison
    let tex_file_path = tex_file.to_path_buf();
    if processed_files.contains(&tex_file_path) {
        return Ok(String::new()); // Skip already processed files to avoid infinite loops
    }
    
    processed_files.push(tex_file_path);
    
    let content = fs::read_to_string(tex_file)?;
    let mut result = content.clone();
    
    // Find all \input and \include commands
    let input_re = Regex::new(r"\\input\{([^}]+)\}")?;
    let include_re = Regex::new(r"\\include\{([^}]+)\}")?;
    
    // Process \input commands
    let mut replacements = Vec::new();
    
    for cap in input_re.captures_iter(&content) {
        let filename = cap.get(1).unwrap().as_str();
        let file_path = resolve_input_path(base_dir, filename)?;
        
        if let Some(path) = file_path {
            included_files.push(path.clone());
            let nested_content = extract_latex_content(base_dir, &path, included_files, processed_files)?;
            replacements.push((cap.get(0).unwrap().as_str().to_string(), nested_content));
        }
    }
    
    // Process \include commands
    for cap in include_re.captures_iter(&content) {
        let filename = cap.get(1).unwrap().as_str();
        let file_path = resolve_input_path(base_dir, filename)?;
        
        if let Some(path) = file_path {
            included_files.push(path.clone());
            let nested_content = extract_latex_content(base_dir, &path, included_files, processed_files)?;
            replacements.push((cap.get(0).unwrap().as_str().to_string(), nested_content));
        }
    }
    
    // Apply all replacements
    for (pattern, replacement) in replacements {
        result = result.replace(&pattern, &replacement);
    }
    
    Ok(result)
}

/// Resolve the path of an input file
pub fn resolve_input_path(base_dir: &Path, filename: &str) -> Result<Option<PathBuf>> {
    let mut file_path = base_dir.join(filename);
    
    // Check with no extension first
    if file_path.exists() {
        return Ok(Some(file_path));
    }
    
    // Try with .tex extension
    if !filename.ends_with(".tex") {
        file_path = base_dir.join(format!("{}.tex", filename));
        if file_path.exists() {
            return Ok(Some(file_path));
        }
    }
    
    log::warn!("Could not find input file: {}", filename);
    Ok(None)
}


pub fn related_work_section(section_title: &str) -> bool {
    let related_work_sections = [
        "related work",
        "background",
        "literature review",
        "prior work",
        "previous work",
        "state of the art",
        "comparison with existing approaches",
        "comparative analysis",
        "context",
        "existing work",
        "existing approaches",
        "existing methods",
        "review of the literature",
        "review of existing work",
        "overview of related work",
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
    
    // Split the content on section commands
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
        let cite_re = Regex::new(r"\\(?:cite|citep|citet|citealp|citeauthor)\{([^}]+)\}")?;
        let mut citations = Vec::new();
        
        for cite_cap in cite_re.captures_iter(&content) {
            let cite_keys = cite_cap.get(1).map_or("", |m| m.as_str());
            for key in cite_keys.split(',') {
                citations.push(key.trim().to_string());
            }
        }
        
        // Remove duplicates
        citations.sort();
        citations.dedup();
        
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
        let cite_re = Regex::new(r"\\(?:cite|citep|citet|citealp|citeauthor)\{([^}]+)\}")?;
        let mut citations = Vec::new();
        
        for cite_cap in cite_re.captures_iter(&content) {
            let cite_keys = cite_cap.get(1).map_or("", |m| m.as_str());
            for key in cite_keys.split(',') {
                citations.push(key.trim().to_string());
            }
        }
        
        // Remove duplicates
        citations.sort();
        citations.dedup();
        
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