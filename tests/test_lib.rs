use bibextract::internal::extract_survey_internal;
use bibextract::error::BibExtractError;
use bibextract::latex::{BibEntry, Bibliography};
use bibextract::internal::format_bibliography_as_bibtex;
use std::collections::HashMap;

#[test]
fn test_extract_survey_internal_no_paper_ids() {
    let paper_ids: Vec<String> = Vec::new();
    let result = extract_survey_internal(paper_ids);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BibExtractError::NoPaperIdsProvided));
}

#[test]
fn test_format_bibliography_as_bibtex() {
    let mut bibliography = Bibliography::new();
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), "Test Title".to_string());
    fields.insert("author".to_string(), "Test Author".to_string());
    fields.insert("raw".to_string(), "@article{testkey, title={Test Title}, author={Test Author}}".to_string());

    let entry = BibEntry {
        key: "testkey".to_string(),
        entry_type: "article".to_string(),
        fields,
    };
    bibliography.insert(entry);

    let bibtex_output = format_bibliography_as_bibtex(&bibliography);
    let expected_output = "@article{author_test_title,\n  author = {Test Author},\n  title = {Test Title},\n}\n\n";
    assert_eq!(bibtex_output, expected_output);
}
