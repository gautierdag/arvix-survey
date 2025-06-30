

#[cfg(test)]
mod bbl_tests {
    use bibextract::latex::Bibliography;
    use std::fs;
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
}
