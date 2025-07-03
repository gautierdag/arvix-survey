use bibextract::latex::{BibEntry, Bibliography};
use std::collections::HashMap;
use serde_json::json;

#[test]
fn test_verify_from_dblp_successful_match() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "test_key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Smith, Alice".to_string()),
            ("title".to_string(), "Machine Learning Advances".to_string()),
            ("year".to_string(), "2023".to_string()),
        ]),
    };

    // Test the scoring logic without actual API calls
    let dblp_results = json!({
        "result": {
            "hits": {
                "@total": "1",
                "hit": [
                    {
                        "info": {
                            "authors": {
                                "author": [{ "text": "Smith, Alice" }]
                            },
                            "title": "Machine Learning Advances",
                            "year": "2023",
                            "venue": "AI Conference",
                            "doi": "10.1000/example"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_some());
    
    let matched_entry = best_match.unwrap();
    assert_eq!(matched_entry.get("title").unwrap().as_str().unwrap(), "Machine Learning Advances");
    assert_eq!(matched_entry.get("year").unwrap().as_str().unwrap(), "2023");
}

#[test]
fn test_find_best_match_title_substring() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Wilson, Robert".to_string()),
            ("title".to_string(), "Deep Learning for Natural Language Processing".to_string()),
            ("year".to_string(), "2023".to_string()),
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
                                "author": [{ "text": "Wilson, Robert" }]
                            },
                            "title": "Deep Learning",
                            "year": "2023"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_some());
}

#[test]
fn test_find_best_match_low_score_threshold() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Anderson, Mark".to_string()),
            ("title".to_string(), "Quantum Computing".to_string()),
            ("year".to_string(), "2022".to_string()),
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
                                "author": [{ "text": "Different, Author" }]
                            },
                            "title": "Completely Different Topic",
                            "year": "2020"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_none());
}

#[test]
fn test_parse_bibtex_multiline_fields() {
    let bib = Bibliography::new();
    let bibtex_str = r#"@article{multiline2024,
            author = {First Author and 
                        Second Author},
            title = {A Very Long Title That Spans
                        Multiple Lines},
            journal = {Journal Name},
            year = {2024}
        }"#;
    let entry = bib.parse_bibtex_entry(bibtex_str);
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.key, "multiline2024");
    assert!(entry.get("title").is_some());
}

#[test]
fn test_parse_bibtex_with_numbers_in_values() {
    let bib = Bibliography::new();
    let bibtex_str = r#"@article{numbers2024,
            author = "Author Name",
            title = "Paper with Numbers 123 and Symbols @#$",
            volume = "42",
            pages = "123--456",
            year = "2024"
        }"#;
    let entry = bib.parse_bibtex_entry(bibtex_str).unwrap();
    assert_eq!(entry.get("volume"), Some(&"42".to_string()));
    assert_eq!(entry.get("pages"), Some(&"123--456".to_string()));
}

#[test]
fn test_find_best_match_multiple_hits_best_score() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Garcia, Maria".to_string()),
            ("title".to_string(), "Artificial Intelligence Applications".to_string()),
            ("year".to_string(), "2023".to_string()),
        ]),
    };

    let dblp_results = json!({
        "result": {
            "hits": {
                "@total": "2",
                "hit": [
                    {
                        "info": {
                            "authors": {
                                "author": [{ "text": "Different, Author" }]
                            },
                            "title": "AI Applications",
                            "year": "2023"
                        }
                    },
                    {
                        "info": {
                            "authors": {
                                "author": [{ "text": "Garcia, Maria" }]
                            },
                            "title": "Artificial Intelligence Applications",
                            "year": "2023"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_some());
    let matched = best_match.unwrap();
    assert_eq!(matched.get("title").unwrap().as_str().unwrap(), "Artificial Intelligence Applications");
}

#[test]
fn test_parse_bibtex_nested_braces() {
    let bib = Bibliography::new();
    let bibtex_str = r#"@inproceedings{nested2024,
            title = {Paper about {Machine Learning} and {Deep {Neural} Networks}},
            author = {Author Name},
            booktitle = {Conference on {AI} and {ML}},
            year = {2024}
        }"#;
    let entry = bib.parse_bibtex_entry(bibtex_str);
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.key, "nested2024");
    assert!(entry.get("title").is_some());
    let title = entry.get("title").unwrap();
    assert!(title.contains("Machine Learning"));
}

#[test]
fn test_find_best_match_author_word_matching() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "John Michael Smith and Mary Elizabeth Johnson".to_string()),
            ("title".to_string(), "Research Study".to_string()),
            ("year".to_string(), "2023".to_string()),
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
                                    { "text": "John Smith" },
                                    { "text": "Mary Johnson" }
                                ]
                            },
                            "title": "Different Study",
                            "year": "2023"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_some());
}

#[test]
fn test_parse_bibtex_empty_fields() {
    let bib = Bibliography::new();
    let bibtex_str = r#"@article{empty2024,
            author = "",
            title = "Valid Title",
            note = {},
            year = "2024"
        }"#;
    let entry = bib.parse_bibtex_entry(bibtex_str).unwrap();
    assert_eq!(entry.get("author"), Some(&"".to_string()));
    assert_eq!(entry.get("note"), Some(&"".to_string()));
    assert_eq!(entry.get("title"), Some(&"Valid Title".to_string()));
}

#[test]
fn test_parse_bibtex_entry() {
    let bib = Bibliography::new();
    let bibtex_str = r#"@inproceedings{test2023,
        author = {Smith, Jane and Brown, Bob},
        title = {Advanced Topics in {Machine Learning}},
        year = {2023},
        booktitle = {Proceedings of AI Conference}
    }"#;
    let entry = bib.parse_bibtex_entry(bibtex_str).unwrap();
    assert_eq!(entry.key, "test2023");
    assert_eq!(entry.entry_type, "inproceedings");
    assert_eq!(entry.get("title"), Some(&"Advanced Topics in {Machine Learning}".to_string()));
    assert_eq!(entry.get("booktitle"), Some(&"Proceedings of AI Conference".to_string()));
}

#[test]
fn test_parse_bibtex_entry_invalid() {
    let bib = Bibliography::new();
    let invalid_bibtex = "not a valid bibtex entry";
    let entry = bib.parse_bibtex_entry(invalid_bibtex);
    assert!(entry.is_none());
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

#[test]
fn test_find_best_match_in_dblp_no_match() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Johnson, Peter".to_string()),
            ("title".to_string(), "A Completely Different Paper".to_string()),
            ("year".to_string(), "2023".to_string()),
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

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_none());
}

#[test]
fn test_find_best_match_partial_title_match() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Doe, John".to_string()),
            ("title".to_string(), "Machine Learning in Computer Vision".to_string()),
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
                                "author": [{ "text": "Doe, John" }]
                            },
                            "title": "Machine Learning in Computer Vision Applications",
                            "year": "2024"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_some());
}

#[test]
fn test_find_best_match_year_mismatch() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Doe, John".to_string()),
            ("title".to_string(), "A Paper".to_string()),
            ("year".to_string(), "2020".to_string()),
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
                                "author": [{ "text": "Smith, Jane" }]
                            },
                            "title": "Different Paper",
                            "year": "2024"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_none());
}

#[test]
fn test_find_best_match_empty_results() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("title".to_string(), "Some Title".to_string()),
            ("year".to_string(), "2024".to_string()),
        ]),
    };

    let dblp_results = json!({
        "result": {
            "hits": {
                "@total": "0",
                "hit": []
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_none());
}

#[test]
fn test_parse_bibtex_mixed_quotes_braces() {
    let bib = Bibliography::new();
    let bibtex_str = r#"@book{mixedtest,
    author = "Author Name",
    title = {Book Title with {Special} Characters},
    publisher = "Publisher Name",
    year = {2024}
    }"#;
    let entry = bib.parse_bibtex_entry(bibtex_str).unwrap();
    assert_eq!(entry.entry_type, "book");
    assert_eq!(entry.get("author"), Some(&"Author Name".to_string()));
    assert_eq!(entry.get("publisher"), Some(&"Publisher Name".to_string()));
}

#[test]
fn test_find_best_match_author_scoring() {
    let bib = Bibliography::new();
    let entry = BibEntry {
        key: "key".to_string(),
        entry_type: "article".to_string(),
        fields: HashMap::from([
            ("author".to_string(), "Alice Smith and Bob Johnson and Carol Williams and David Brown and Eve Davis".to_string()),
            ("title".to_string(), "Research Paper".to_string()),
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
                                    { "text": "Alice Smith" },
                                    { "text": "Bob Johnson" },
                                    { "text": "Carol Williams" }
                                ]
                            },
                            "title": "Different Research Paper",
                            "year": "2024"
                        }
                    }
                ]
            }
        }
    });

    let best_match = bib.find_best_match_in_dblp(&dblp_results, &entry);
    assert!(best_match.is_some());

    // Verify the entry fields are as expected
    assert_eq!(entry.key, "key");
    assert_eq!(entry.entry_type, "article");
    assert_eq!(entry.get("author"), Some(&"Alice Smith and Bob Johnson and Carol Williams and David Brown and Eve Davis".to_string()));
    assert_eq!(entry.get("title"), Some(&"Research Paper".to_string()));
    assert_eq!(entry.get("year"), Some(&"2024".to_string()));
}