use bibextract::latex::{BibEntry, Bibliography};
use std::collections::HashMap;
use serde_json::json;

#[test]
fn test_verify_entry() {
    let bib = Bibliography::new();
    let mut entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Doe, John".to_string()),
            ("title".to_string(), "A Title".to_string()),
            ("year".to_string(), "2024".to_string()),
        ]),
    };

    let result = bib.verify_entry(&mut entry);
    assert!(result.is_ok());
    assert!(entry.get("verified_source").is_some());
}

#[test]
fn test_parse_bibtex_entry() {
    let bib = Bibliography::new();
    let bibtex_str = r#"@article{key,
    author = "Doe, John",
    title = "A Title",
    year = "2024"
}"#;
    let entry = bib.parse_bibtex_entry(bibtex_str).unwrap();
    assert_eq!(entry.key, "key");
    assert_eq!(entry.entry_type, "article");
    assert_eq!(entry.get("author"), Some(&"Doe, John".to_string()));
    assert_eq!(entry.get("title"), Some(&"A Title".to_string()));
    assert_eq!(entry.get("year"), Some(&"2024".to_string()));
}

#[test]
fn test_find_best_match_in_dblp() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Doe, John and Smith, Jane".to_string()),
            ("title".to_string(), "A Great Paper on Science".to_string()),
            ("year".to_string(), "2024".to_string()),
        ]),
    };

    let dblp_results = json!({
        "result": {
            "hits": {
                "@total": "1",
                "hit": [
                    {
                        "info": {
                            "authors": {
                                "author": [
                                    { "text": "Doe, John" },
                                    { "text": "Smith, Jane" }
                                ]
                            },
                            "title": "A Great Paper on Science",
                            "year": "2024",
                            "venue": "A Prestigious Journal",
                            "url": "https://example.com/paper",
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry).unwrap();
    assert_eq!(best_match.get("title").unwrap().as_str().unwrap(), "A Great Paper on Science");
    assert_eq!(best_match.get("year").unwrap().as_str().unwrap(), "2024");
}