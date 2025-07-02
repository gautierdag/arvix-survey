use bibextract::latex::bibliography::{BibEntry, Bibliography};
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_bib_entry_builder() {
    let entry = BibEntry::builder("key1", "article")
        .field("title", "Test Title")
        .field("author", "Test Author")
        .build();

    assert_eq!(entry.key, "key1");
    assert_eq!(entry.entry_type, "article");
    assert_eq!(entry.get("title").unwrap(), "Test Title");
    assert_eq!(entry.get("author").unwrap(), "Test Author");
    assert!(entry.get("nonexistent").is_none());

    let mut fields = HashMap::new();
    fields.insert("journal".to_string(), "Test Journal".to_string());
    let entry_with_fields = BibEntry::builder("key2", "book")
        .fields(fields.into_iter())
        .build();
    assert_eq!(entry_with_fields.get("journal").unwrap(), "Test Journal");
}

#[test]
fn test_bib_entry_set_get() {
    let mut entry = BibEntry::new("key3".to_string(), "inproceedings".to_string());
    entry.set("year", "2023".to_string());
    assert_eq!(entry.get("year").unwrap(), "2023");
}

#[test]
fn test_bibliography_methods() {
    let mut bib = Bibliography::new();
    assert!(bib.entries.is_empty());


    let entry1 = BibEntry::builder("key1", "article").build();
    bib.insert(entry1.clone());
    assert_eq!(bib.entries.len(), 1);
    assert!(bib.get("key1").is_some());

    let entry2 = BibEntry::builder("key2", "book").build();
    bib.insert(entry2.clone());
    assert_eq!(bib.entries.len(), 2);

    let mut iter_keys: Vec<String> = bib.iter().map(|e| e.key.clone()).collect();
    iter_keys.sort();
    assert_eq!(iter_keys, vec!["key1".to_string(), "key2".to_string()]);
}


#[test]
fn test_parse_bibliography_files() {
    let dir = tempdir().unwrap();
    let bbl_file1 = dir.path().join("test1.bbl");
    let bbl_file2 = dir.path().join("test2.bbl");

    fs::write(&bbl_file1, r#"\begin{thebibliography}{99}
\bibitem{key1}
Author A. (2020). Title A.
\end{thebibliography}"#).unwrap();
    fs::write(&bbl_file2, r#"\begin{thebibliography}{99}
\bibitem{key2}
Author B. (2021). Title B.
\end{thebibliography}"#).unwrap();

    let bib_files = vec![bbl_file1, bbl_file2];
    let bib = Bibliography::parse_bibliography_files(&bib_files).unwrap();
    assert_eq!(bib.entries.len(), 2);
    assert!(bib.get("key1").is_some());
    assert!(bib.get("key2").is_some());
}

#[test]
fn test_normalize_citation_key() {
    let bib = Bibliography::new();

    let entry1 = BibEntry::builder("key1", "article")
        .field("author", "John Doe")
        .field("title", "A Test Paper on Something")
        .field("year", "2023")
        .build();
    assert_eq!(bib.normalize_citation_key(&entry1), "doe_test_paper_something_2023");

    let entry2 = BibEntry::builder("key2", "book")
        .field("author", "Jane A. Smith and Bob Johnson")
        .field("title", "Another Study")
        .build();
    assert_eq!(bib.normalize_citation_key(&entry2), "smith_another_study");

    let entry3 = BibEntry::builder("key3", "misc")
        .field("author", "John et al.")
        .field("title", "Short")
        .build();
    assert_eq!(bib.normalize_citation_key(&entry3), "john_short");
}

#[test]
fn test_extract_arxiv_id() {
    let bib = Bibliography::new();

    let entry1 = BibEntry::builder("key1", "article")
        .field("title", "arXiv:2305.15030 Some Title")
        .build();
    assert_eq!(bib.extract_arxiv_id(&entry1).unwrap(), "2305.15030");

    let entry2 = BibEntry::builder("key2", "article")
        .field("journal", "arXiv:2305.15030v1")
        .build();
    assert_eq!(bib.extract_arxiv_id(&entry2).unwrap(), "2305.15030");

    let entry3 = BibEntry::builder("key3", "article")
        .field("note", "Some note with arXiv:2305.15030 in it")
        .build();
    assert_eq!(bib.extract_arxiv_id(&entry3).unwrap(), "2305.15030");

    let entry4 = BibEntry::builder("key4", "article")
        .field("raw", r"\bibitem{key4} Some raw text with arXiv:2305.15030").build();
    assert_eq!(bib.extract_arxiv_id(&entry4).unwrap(), "2305.15030");

    let entry5 = BibEntry::builder("2305.15030", "article").build();
    assert_eq!(bib.extract_arxiv_id(&entry5).unwrap(), "2305.15030");

    let entry6 = BibEntry::builder("key6", "article").build();
    assert!(bib.extract_arxiv_id(&entry6).is_none());
}

// Mock reqwest for get_arxiv_bibtex test
// This requires a more advanced mocking setup or a separate integration test
// #[test]
// fn test_get_arxiv_bibtex() {
//     // Mock HTTP response for arXiv API
//     // This is complex and might require a dedicated mocking library or approach
// }

#[test]
fn test_normalize_citations() {
    let mut bib = Bibliography::new();
    let entry1 = BibEntry::builder("oldkey1", "article")
        .field("author", "Author One")
        .field("title", "Title Title")
        .field("year", "2020")
        .build();
    bib.insert(entry1);

    let entry2 = BibEntry::builder("oldkey2", "book")
        .field("author", "Author Two")
        .field("title", "Title Title")
        .field("year", "2021")
        .build();
    bib.insert(entry2);

    let content = r"This is some text with citations \cite{oldkey1} and \cite{oldkey2, unknownkey}.";
    let (normalized_content, key_map) = bib.normalize_citations(content).unwrap();

    assert!(normalized_content.contains(r"\cite{one_title_title_2020}"));
    assert!(normalized_content.contains(r"\cite{two_title_title_2021, unknownkey}"));
    assert_eq!(key_map.get("oldkey1").unwrap(), "one_title_title_2020");
    assert_eq!(key_map.get("oldkey2").unwrap(), "two_title_title_2021");
    assert!(key_map.get("unknownkey").is_none());
}
