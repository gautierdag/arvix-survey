#[cfg(test)]
mod tests {
    use bibextract::latex::{BibEntry, Bibliography, find_bbl_files, resolve_input_path};
    use std::fs::{self, File};
    use tempfile::tempdir;
    use std::path::Path;

    fn load_bbl_fixture(file_name: &str) -> String {
        let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        path.push("tests");
        path.push("bbls");
        path.push(file_name);
        fs::read_to_string(&path).expect(&format!("Failed to read {}", file_name))
    }

    #[test]
    fn test_bib_entry_builder() {
        let entry = BibEntry::builder("key", "article")
            .field("author", "Doe, John")
            .field("title", "A Title")
            .field("year", "2024")
            .build();

        assert_eq!(entry.key, "key");
        assert_eq!(entry.entry_type, "article");
        assert_eq!(entry.get("author"), Some(&"Doe, John".to_string()));
        assert_eq!(entry.get("title"), Some(&"A Title".to_string()));
        assert_eq!(entry.get("year"), Some(&"2024".to_string()));
    }

    #[test]
    fn test_normalize_citation_key() {
        let mut bib = Bibliography::new();
        let entry = BibEntry::builder("key", "article")
            .field("author", "Doe, John and Smith, Jane")
            .field("title", "A Great Paper on Science")
            .field("year", "2024")
            .build();
        bib.insert(entry.clone());

        let normalized_key = bib.normalize_citation_key(&entry);
        assert_eq!(normalized_key, "doe_great_paper_science_2024");
    }

    #[test]
    fn test_extract_arxiv_id() {
        let mut bib = Bibliography::new();
        let entry = BibEntry::builder("key", "article")
            .field("title", "A Paper with arXiv:2401.12345")
            .build();
        bib.insert(entry.clone());

        let arxiv_id = bib.extract_arxiv_id(&entry);
        assert_eq!(arxiv_id, Some("2401.12345".to_string()));
    }

    #[test]
    fn test_normalize_citations() {
        let mut bib = Bibliography::new();
        let entry = BibEntry::builder("key1", "article")
            .field("author", "Doe, John")
            .field("title", "A Title")
            .field("year", "2024")
            .build();
        bib.insert(entry);

        let content = r"\cite{key1}";
        let (normalized_content, key_map) = bib.normalize_citations(content).unwrap();

        assert_eq!(normalized_content, r"\cite{doe_title_2024}");
        assert_eq!(key_map.get("key1"), Some(&"doe_title_2024".to_string()));
    }

    #[test]
    fn test_find_bbl_files() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("test.bbl")).unwrap();
        File::create(dir.path().join("other.txt")).unwrap();
        let sub_dir = dir.path().join("sub");
        fs::create_dir(&sub_dir).unwrap();
        File::create(sub_dir.join("another.bbl")).unwrap();

        let bbl_files = find_bbl_files(dir.path()).unwrap();
        assert_eq!(bbl_files.len(), 2);
    }

    #[test]
    fn test_resolve_input_path() {
        let dir = tempdir().unwrap();
        let tex_path = dir.path().join("included.tex");
        File::create(&tex_path).unwrap();

        // Test with extension
        let resolved_path = resolve_input_path(dir.path(), "included.tex").unwrap();
        assert_eq!(resolved_path, Some(tex_path.clone()));

        // Test without extension
        let resolved_path_no_ext = resolve_input_path(dir.path(), "included").unwrap();
        assert_eq!(resolved_path_no_ext, Some(tex_path.clone()));
    }

    #[test]
    fn test_extract_sections() {
        let content = r#"
\section{Introduction}
This is the introduction.
\subsection{Background}
Some background information.
\section{Conclusion}
This is the conclusion.
"#;
        let bibliography = Bibliography::new();
        let sections = bibextract::latex::citation::extract_sections_from_latex(content, &bibliography).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].title, "Background");
    }

    #[test]
    fn test_parse_example_bbl_1() {
        let bbl_content = load_bbl_fixture("1.bbl");
        let bibliography = Bibliography::parse_bbl(&bbl_content).expect("Failed to parse example.bbl");

        assert!(bibliography.get("vpt").is_some(), "Entry 'vpt' should be parsed");
        assert!(bibliography.get("babyai_iclr19").is_some(), "Entry 'babyai_iclr19' should be parsed");
        assert!(bibliography.get("deng2023mind2web").is_some(), "Entry 'deng2023mind2web' should be parsed");
        assert!(bibliography.get("llama3").is_some(), "Entry 'llama3' should be parsed");

        let vpt_entry = bibliography.get("vpt").unwrap();
        assert_eq!(vpt_entry.get("year").unwrap(), "2022");
        assert!(vpt_entry.get("author").unwrap().contains("Baker"));
    }

    #[test]
    fn test_parse_example_bbl_2() {
        let bbl_content = load_bbl_fixture("2.bbl");
        let bibliography = Bibliography::parse_bbl(&bbl_content).expect("Failed to parse example2.bbl");

        assert!(bibliography.get("acemoglu2018artificial").is_some(), "Entry 'acemoglu2018artificial' should be parsed");
        assert!(bibliography.get("gqa2023").is_some(), "Entry 'gqa2023' should be parsed");
        assert!(bibliography.get("falcon40b").is_some(), "Entry 'falcon40b' should be parsed");
        assert!(bibliography.get("zhuo2023exploring").is_some(), "Entry 'zhuo2023exploring' should be parsed");

        let acemoglu_entry = bibliography.get("acemoglu2018artificial").unwrap();
        assert_eq!(acemoglu_entry.get("year").unwrap(), "2018");
        assert!(acemoglu_entry.get("author").unwrap().contains("Acemoglu"));
    }

    #[test]
    fn test_parse_example_bbl_3() {
        let bbl_content = load_bbl_fixture("3.bbl");
        let bibliography = Bibliography::parse_bbl(&bbl_content).expect("Failed to parse 3.bbl");

        // Test that key entries are parsed
        assert!(bibliography.get("wei2022chain").is_some(), "Entry 'wei2022chain' should be parsed");
        assert!(bibliography.get("kojima2022large").is_some(), "Entry 'kojima2022large' should be parsed");
        assert!(bibliography.get("shunyu2024tree").is_some(), "Entry 'shunyu2024tree' should be parsed");
        assert!(bibliography.get("besta2024graph").is_some(), "Entry 'besta2024graph' should be parsed");
        assert!(bibliography.get("huang22a").is_some(), "Entry 'huang22a' should be parsed");

        // Test specific author parsing for wei2022chain entry
        let wei_entry = bibliography.get("wei2022chain").unwrap();
        assert_eq!(wei_entry.get("year").unwrap(), "2022");
        let wei_author = wei_entry.get("author").unwrap();
        assert!(wei_author.contains("Wei"), "Author should contain 'Wei'");
        assert!(wei_author.contains("Wang"), "Author should contain 'Wang'");
        assert!(wei_author.contains("Schuurmans"), "Author should contain 'Schuurmans'");
        
        // Test title parsing for wei2022chain entry
        assert!(wei_entry.get("title").is_some(), "Title should be present for wei_entry entry");
        if let Some(title) = wei_entry.get("title") {
            assert!(title.contains("Chain-of-thought") || title.contains("chain"), 
                "Title should contain reference to chain-of-thought");
        }

        // Test specific author parsing for kojima2022large entry
        let kojima_entry = bibliography.get("kojima2022large").unwrap();
        assert_eq!(kojima_entry.get("year").unwrap(), "2022");
        let kojima_author = kojima_entry.get("author").unwrap();
        assert!(kojima_author.contains("Kojima"), "Author should contain 'Kojima'");
        assert!(kojima_author.contains("Gu"), "Author should contain 'Gu'");
        assert!(kojima_author.contains("Reid"), "Author should contain 'Reid'");

        // assert title exists
        assert!(kojima_entry.get("title").is_some(), "Title should be present for kojima2022large entry");

        // Test title parsing for kojima2022large entry  
        if let Some(title) = kojima_entry.get("title") {
            assert!(title.contains("Large language models") || title.contains("zero-shot"), 
                "Title should contain reference to large language models or zero-shot");
        }

        // Test specific author parsing for besta2024graph entry
        let besta_entry = bibliography.get("besta2024graph").unwrap();
        assert_eq!(besta_entry.get("year").unwrap(), "2024");
        let besta_author = besta_entry.get("author").unwrap();
        assert!(besta_author.contains("Besta"), "Author should contain 'Besta'");
        assert!(besta_author.contains("Blach"), "Author should contain 'Blach'");

        // assert title exists
        assert!(besta_entry.get("title").is_some(), "Title should be present for besta2024graph entry");

        // Test title parsing for besta2024graph entry
        if let Some(title) = besta_entry.get("title") {
            assert!(title.contains("Graph of thoughts") || title.contains("graph"), 
                "Title should contain reference to graph of thoughts");
        }
    }
}