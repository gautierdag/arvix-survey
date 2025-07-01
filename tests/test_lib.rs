use bibextract::internal::extract_survey_internal;
use bibextract::error::BibExtractError;

#[test]
fn test_extract_survey_internal_no_paper_ids() {
    let paper_ids: Vec<String> = Vec::new();
    let result = extract_survey_internal(paper_ids);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BibExtractError::NoPaperIdsProvided));
}
