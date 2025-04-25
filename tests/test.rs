#[cfg(test)]
mod tests {
    use arvix_survey::latex::{Bibliography, BibEntry};
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_bib_entry() {
        // Test BibEntry creation and field manipulation
        let mut entry = BibEntry::new("test_key".to_string(), "article".to_string());
        
        // Test setting and getting fields
        entry.set("author", "Test Author".to_string());
        entry.set("title", "Test Title".to_string());
        entry.set("year", "2024".to_string());
        
        assert_eq!(entry.get("author"), Some(&"Test Author".to_string()));
        assert_eq!(entry.get("title"), Some(&"Test Title".to_string()));
        assert_eq!(entry.get("year"), Some(&"2024".to_string()));
        assert_eq!(entry.get("nonexistent"), None);
        
        // Test key and entry_type
        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.entry_type, "article");
    }
    
    #[test]
    fn test_bibliography_basics() {
        let mut bib = Bibliography::new();
        
        // Create and add an entry
        let entry1 = BibEntry::new("key1".to_string(), "article".to_string());
        let mut entry2 = BibEntry::new("key2".to_string(), "book".to_string());
        entry2.set("author", "Author2".to_string());
        
        bib.insert(entry1);
        bib.insert(entry2);
        
        // Test retrieval
        assert!(bib.get("key1").is_some());
        assert!(bib.get("key2").is_some());
        assert!(bib.get("nonexistent").is_none());
        
        // Test iterator
        let entries: Vec<_> = bib.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_bbl_parser() {
        // Test the BBL parser with a sample BBL content
        let bbl_content = r#"
\begin{thebibliography}{}

\bibitem[\protect\citeauthoryear{Baker \bgroup \em et al.\egroup }{2022}]{vpt}
Bowen Baker, Ilge Akkaya, Peter Zhokov, Joost Huizinga, Jie Tang, Adrien Ecoffet, Brandon Houghton, Raul Sampedro, and Jeff Clune.
\newblock Video pretraining (vpt): Learning to act by watching unlabeled online videos.
\newblock In S.~Koyejo, S.~Mohamed, A.~Agarwal, D.~Belgrave, K.~Cho, and A.~Oh, editors, {\em Advances in Neural Information Processing Systems}, volume~35, pages 24639--24654. Curran Associates, Inc., 2022.

\bibitem[\protect\citeauthoryear{Chevalier-Boisvert \bgroup \em et al.\egroup }{2019}]{babyai_iclr19}
Maxime Chevalier-Boisvert, Dzmitry Bahdanau, Salem Lahlou, Lucas Willems, Chitwan Saharia, Thien~Huu Nguyen, and Yoshua Bengio.
\newblock Baby{AI}: First steps towards grounded language learning with a human in the loop.
\newblock In {\em International Conference on Learning Representations}, 2019.

\bibitem[\protect\citeauthoryear{Deng \bgroup \em et al.\egroup }{2023}]{deng2023mind2web}
Xiang Deng, Yu~Gu, Boyuan Zheng, Shijie Chen, Samuel Stevens, Boshi Wang, Huan Sun, and Yu~Su.
\newblock Mind2web: Towards a generalist agent for the web, 2023.

\bibitem[\protect\citeauthoryear{Dubey \bgroup \em et al.\egroup }{2024}]{llama3}
Abhimanyu Dubey, Abhinav Jauhri, Abhinav Pandey, Abhishek Kadian, Ahmad Al-Dahle, Aiesha Letman, Akhil Mathur, Alan Schelten, Amy Yang, Angela Fan, et~al.
\newblock The llama 3 herd of models.
\newblock {\em arXiv preprint arXiv:2407.21783}, 2024.

\end{thebibliography}
"#;
        
        // Parse the BBL content
        let bibliography = Bibliography::parse_bbl(bbl_content).expect("Failed to parse BBL");

        // Verify entries were properly parsed
        assert!(bibliography.get("vpt").is_some());
        assert!(bibliography.get("babyai_iclr19").is_some());
        assert!(bibliography.get("deng2023mind2web").is_some());
        assert!(bibliography.get("llama3").is_some());
        
        // Verify specific fields from entries
        let vpt_entry = bibliography.get("vpt").unwrap();
        assert!(vpt_entry.get("author").is_some());
        assert_eq!(vpt_entry.get("year"), Some(&"2022".to_string()));
        
        let babyai_entry = bibliography.get("babyai_iclr19").unwrap();
        assert!(babyai_entry.get("author").is_some());
        assert_eq!(babyai_entry.get("year"), Some(&"2019".to_string()));
        
        let deng_entry = bibliography.get("deng2023mind2web").unwrap();
        assert_eq!(deng_entry.get("year"), Some(&"2023".to_string()));
        
        let llama_entry = bibliography.get("llama3").unwrap();
        assert_eq!(llama_entry.get("year"), Some(&"2024".to_string()));
    }

    #[test]
    fn test_normalize_citation_key() {
        let mut bib = Bibliography::new();
        
        // Create test entries
        let mut entry1 = BibEntry::new("key1".to_string(), "article".to_string());
        entry1.set("author", "John Smith and Jane Doe".to_string());
        entry1.set("year", "2020".to_string());
        entry1.set("title", "Important Discoveries in Science".to_string());
        
        let mut entry2 = BibEntry::new("key2".to_string(), "article".to_string());
        entry2.set("author", "Brown, Robert".to_string());
        entry2.set("year", "2019".to_string());
        entry2.set("title", "A New Approach".to_string());
        
        bib.insert(entry1.clone());
        bib.insert(entry2.clone());
        
        // Test normalization
        let normalized1 = bib.normalize_citation_key(&entry1);
        let normalized2 = bib.normalize_citation_key(&entry2);
        
        // Check if normalized keys include author name and year
        assert!(normalized1.contains("smith"));
        assert!(normalized1.contains("2020"));
        assert!(normalized1.contains("important") || normalized1.contains("discoveries"));
        
        assert!(normalized2.contains("brown"));
        assert!(normalized2.contains("2019"));
        assert!(normalized2.contains("approach"));
        
        // Test entry with missing fields
        let mut entry3 = BibEntry::new("key3".to_string(), "article".to_string());
        entry3.set("author", "Lee, Ann".to_string());
        // No year or title
        
        let normalized3 = bib.normalize_citation_key(&entry3);
        assert!(normalized3.contains("lee"));
        assert!(!normalized3.contains("_20"));  // No year
    }

    #[test]
    fn test_normalize_citations() {
        let mut bib = Bibliography::new();
        
        // Add entries to bibliography
        let mut entry1 = BibEntry::new("smith2020".to_string(), "article".to_string());
        entry1.set("author", "John Smith".to_string());
        entry1.set("year", "2020".to_string());
        entry1.set("title", "Important Research".to_string());
        
        let mut entry2 = BibEntry::new("jones2019".to_string(), "article".to_string());
        entry2.set("author", "Alice Jones".to_string());
        entry2.set("year", "2019".to_string());
        entry2.set("title", "Groundbreaking Study".to_string());
        
        bib.insert(entry1);
        bib.insert(entry2);
        
        // Test LaTeX content with citations
        let content = r"
        \section{Introduction}
        This is a reference \cite{smith2020} in text.
        Multiple references \citep{smith2020,jones2019} should also work.
        References in different formats: \citet{smith2020}, \citealp{jones2019}.
        Missing reference \cite{unknown} should remain unchanged.
        ";
        
        let (normalized_content, key_map) = bib.normalize_citations(content).expect("Failed to normalize citations");
        
        // Verify the key mapping
        assert!(key_map.contains_key("smith2020"));
        assert!(key_map.contains_key("jones2019"));
        assert_eq!(key_map.len(), 2);
        
        // Check that citations were replaced
        let normalized_smith_key = key_map.get("smith2020").unwrap();
        let normalized_jones_key = key_map.get("jones2019").unwrap();
        
        assert!(normalized_content.contains(&format!("\\cite{{{}}}", normalized_smith_key)));
        assert!(normalized_content.contains(&format!("\\citep{{{}, {}}}", normalized_smith_key, normalized_jones_key)));
        assert!(normalized_content.contains(&format!("\\citet{{{}}}", normalized_smith_key)));
        assert!(normalized_content.contains(&format!("\\citealp{{{}}}", normalized_jones_key)));
        
        // Check that unknown reference remained unchanged
        assert!(normalized_content.contains("\\cite{unknown}"));
    }

    #[test]
    fn test_find_bbl_files() {
        use arvix_survey::latex::find_bbl_files;
        
        // Create a temporary directory with some BBL files
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();
        
        // Create directory structure
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdir");
        
        // Create test files
        fs::write(temp_path.join("main.bbl"), "bbl content").expect("Failed to write test file");
        fs::write(temp_dir.path().join("other.tex"), "tex content").expect("Failed to write test file");
        fs::write(sub_dir.join("nested.bbl"), "nested bbl content").expect("Failed to write test file");
        
        // Find BBL files
        let bbl_files = find_bbl_files(temp_path).expect("Failed to find BBL files");
        
        // Verify results
        assert_eq!(bbl_files.len(), 2);
        assert!(bbl_files.iter().any(|path| path.file_name().unwrap() == "main.bbl"));
        assert!(bbl_files.iter().any(|path| path.file_name().unwrap() == "nested.bbl"));
    }

    #[test]
    fn test_resolve_input_path() {
        use arvix_survey::latex::resolve_input_path;
        
        // Create a temporary directory with some LaTeX files
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();
        
        // Create test files
        fs::write(temp_path.join("file1.tex"), "content").expect("Failed to write test file");
        fs::write(temp_path.join("file2"), "content without extension").expect("Failed to write test file");
        
        // Test with existing file with extension
        let result1 = resolve_input_path(temp_path, "file1.tex").expect("Failed to resolve path");
        assert!(result1.is_some());
        assert_eq!(result1.unwrap().file_name().unwrap(), "file1.tex");
        
        // Test with existing file without extension
        let result2 = resolve_input_path(temp_path, "file2").expect("Failed to resolve path");
        assert!(result2.is_some());
        assert_eq!(result2.unwrap().file_name().unwrap(), "file2");
        
        // Test with extension automatically added
        let result3 = resolve_input_path(temp_dir.path(), "file1").expect("Failed to resolve path");
        assert!(result3.is_some());
        assert_eq!(result3.unwrap().file_name().unwrap(), "file1.tex");
        
        // Test with nonexistent file
        let result4 = resolve_input_path(temp_path, "nonexistent").expect("Failed to resolve path");
        assert!(result4.is_none());
    }

    // Helper function to create temporary LaTeX files for testing
    fn setup_latex_files(base_dir: &Path) -> PathBuf {
        // Create main.tex
        let main_content = r#"
\documentclass{article}
\begin{document}
Main file content
\input{section1}
\input{section2.tex}
\include{section3}
\end{document}
"#;
        let main_file = base_dir.join("main.tex");
        fs::write(&main_file, main_content).expect("Failed to write main.tex");
        
        // Create section files
        fs::write(base_dir.join("section1.tex"), "Section 1 content").expect("Failed to write section1.tex");
        fs::write(base_dir.join("section2.tex"), "Section 2 content").expect("Failed to write section2.tex");
        fs::write(base_dir.join("section3.tex"), "Section 3 content").expect("Failed to write section3.tex");
        
        main_file
    }

    #[test]
    fn test_extract_all_latex_from_files() {
        use arvix_survey::latex::extract_all_latex_from_files;
        
        // Create a temporary directory with LaTeX files
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();
        
        // Setup test files
        let main_file = setup_latex_files(temp_path);
        
        // Extract all LaTeX content
        let (content, included_files) = extract_all_latex_from_files(temp_path, &main_file)
            .expect("Failed to extract LaTeX content");
        
        // Verify the content includes everything
        assert!(content.contains("Main file content"));
        assert!(content.contains("Section 1 content"));
        assert!(content.contains("Section 2 content"));
        assert!(content.contains("Section 3 content"));
        
        // Verify we tracked all included files
        assert_eq!(included_files.len(), 3);
    }
}