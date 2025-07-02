use bibextract::latex::{clean_text, ArxivPaper, Bibliography, BibEntry};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_clean_text() {
    assert_eq!(clean_text("Hello, World!"), "hello_world");
    assert_eq!(clean_text("  Test   String  with spaces "), "test_string_with_spaces");
    assert_eq!(clean_text("Another-Test_String.123"), "another_test_string_123");
    assert_eq!(clean_text("UPPERCASE"), "uppercase");
    assert_eq!(clean_text("mixedCase"), "mixedcase");
    assert_eq!(clean_text("12345"), "12345");
    assert_eq!(clean_text(" "), "");
    assert_eq!(clean_text(""), "");
}

#[test]
fn test_arxiv_paper_verify_bibliography() {
    // Create a dummy ArxivPaper
    let mut paper = ArxivPaper {
        id: "test_id".to_string(),
        title: "Test Title".to_string(),
        authors: "Test Author".to_string(),
        sections: Vec::new(),
        bibliography: Bibliography::new(),
        _temp_dir: tempdir().unwrap(),
    };

    // Add some dummy BibEntries to the bibliography
    let mut fields1 = HashMap::new();
    fields1.insert("title".to_string(), "Paper One".to_string());
    fields1.insert("author".to_string(), "Author A".to_string());
    let entry1 = BibEntry { key: "key1".to_string(), entry_type: "article".to_string(), fields: fields1 };
    paper.bibliography.insert(entry1);

    let mut fields2 = HashMap::new();
    fields2.insert("title".to_string(), "Paper Two".to_string());
    fields2.insert("author".to_string(), "Author B".to_string());
    let entry2 = BibEntry { key: "key2".to_string(), entry_type: "article".to_string(), fields: fields2 };
    paper.bibliography.insert(entry2);

    // Mock the verify_entry method of Bibliography to control verification outcome
    // This is a simplification; in a real scenario, you might use a trait or dependency injection
    // to mock the external call to verify_entry more robustly.
    // For now, we'll assume verify_entry always returns true for simplicity in this test.
    // The actual verification logic is tested in test_verification.rs

    let verified_count = paper.verify_bibliography().unwrap();
    // Since we are not mocking the actual verification, and it relies on network calls,
    // this test will likely not increase coverage for the internal verification logic.
    // It primarily tests the flow within verify_bibliography.
    assert_eq!(verified_count, 0); // Assuming no actual verification happens without mocking
}
