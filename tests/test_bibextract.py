import pytest
from bibextract import extract_survey


# Basic test case with a known arXiv paper
def test_extract_survey_single_paper():
    # A known paper with a .bbl file and a "Related work" section.
    paper_ids = ["1912.02292"]
    result = extract_survey(paper_ids)

    assert "survey_text" in result
    assert "bibtex" in result
    assert isinstance(result["survey_text"], str)
    assert isinstance(result["bibtex"], str)

    # Check for a known section and a known author in the bibliography.
    assert "Related work" in result["survey_text"]
    assert "Belkin" in result["bibtex"]


# Test with multiple papers to check bibliography consolidation
def test_extract_survey_multiple_papers():
    paper_ids = ["1912.02292", "2308.06391"]  # Both have .bbl files
    result = extract_survey(paper_ids)

    assert "survey_text" in result
    assert "bibtex" in result

    # Check that authors from both bibliographies are present.
    assert "Belkin" in result["bibtex"]  # From 1912.02292
    assert "Hoffmann" in result["bibtex"]  # From 2104.08653


# Test with an invalid arXiv ID
def test_extract_survey_invalid_id():
    with pytest.raises(RuntimeError, match=r"Error processing arXiv papers: .*"):
        extract_survey(["invalid-id"])


# Test with an empty list of paper IDs
def test_extract_survey_empty_list():
    with pytest.raises(
        RuntimeError, match=r"Error processing arXiv papers: No paper IDs provided"
    ):
        extract_survey([])
