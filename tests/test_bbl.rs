use bibextract::latex::{Bibliography};
use std::fs::{self};
use std::path::Path;

fn load_bbl_fixture(file_name: &str) -> String {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    path.push("tests");
    path.push("bbls");
    path.push(file_name);
    fs::read_to_string(&path).expect(&format!("Failed to read {}", file_name))
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