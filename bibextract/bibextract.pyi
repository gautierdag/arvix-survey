from typing import List, TypedDict

class ExtractResult(TypedDict):
    """
    A dictionary containing the extracted survey text and BibTeX bibliography.
    """

    survey_text: str
    bibtex: str

def extract_survey(paper_ids: List[str]) -> ExtractResult:
    """
    Processes one or more arXiv papers to extract survey content and a consolidated BibTeX bibliography.

    Args:
        paper_ids: A list of arXiv paper IDs (e.g., ['2104.08653', '2203.15556']).

    Returns:
        A dictionary with two keys:
        - "survey_text": A string containing the extracted LaTeX sections.
        - "bibtex": A string containing the consolidated BibTeX bibliography.

    Raises:
        RuntimeError: If there is an error processing the papers, such as network issues,
                      missing files, or parsing failures.
    """
    ...
