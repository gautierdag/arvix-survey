use pyo3::prelude::*;
use pyo3::types::PyDict;
pub mod latex;
pub mod error;
pub use internal::extract_survey_internal;


/// A Python module implemented in Rust.
#[pymodule]
fn bibextract(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(extract_survey, m)?)?;
    Ok(())
}

/// Processes one or more arXiv papers to extract survey content and a consolidated BibTeX bibliography.
#[pyfunction]
#[pyo3(name = "extract_survey")]
fn extract_survey(paper_ids: Vec<String>) -> PyResult<PyObject> {
    // // Initialize logging with a minimal level to avoid spamming Python applications.
    // let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).try_init();
    Python::with_gil(|py| {
        // Process papers and handle any errors, converting them to Python exceptions.
        match internal::extract_survey_internal(paper_ids) {
            Ok((survey_text, bibtex)) => {
                let dict = PyDict::new(py);
                dict.set_item("survey_text", survey_text)?;
                dict.set_item("bibtex", bibtex)?;
                Ok(dict.into())
            }
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Error processing arXiv papers: {}",
                e
            ))),
        }
    })
}

// Encapsulate the core logic in a private module to be shared between the lib and bin targets.
pub mod internal {
    use crate::latex;
    use crate::error::BibExtractError;
    use log::info;
    use rayon::prelude::*;

    /// Internal function to process papers and return survey text and BibTeX.
    /// This is the core logic shared by the Python API and the CLI.
    pub fn extract_survey_internal(paper_ids: Vec<String>) -> Result<(String, String), BibExtractError> {
        use latex::Bibliography;

        if paper_ids.is_empty() {
            return Err(BibExtractError::NoPaperIdsProvided);
        }

        let all_papers: Vec<_> = paper_ids
            .par_iter()
            .with_max_len(4) // Limit parallel jobs to 4
            .map(|paper_id| {
                info!("Processing arXiv paper with ID: {}", paper_id);
                let mut paper = latex::download_arxiv_source(paper_id)?;

                info!("Verifying bibliography entries for paper {}", paper_id);
                let verified_count = paper.verify_bibliography()?;
                info!(
                    "Verified {}/{} entries for paper {} using parallel verification",
                    verified_count,
                    paper.bibliography.iter().count(),
                    paper_id
                );

                info!("Found {} sections with bibliography entries", paper.sections.len());
                Ok(paper)
            })
            .collect::<Result<Vec<_>, BibExtractError>>()?;

        let mut consolidated_bibliography = Bibliography::new();
        // Merge bibliographies from all papers into a single consolidated one.
        for paper in &all_papers {
            for entry in paper.bibliography.iter() {
                consolidated_bibliography.insert(entry.clone());
            }
        }

        // Process and format all sections, normalizing citations against the consolidated bibliography.
        let mut survey_text = String::new();
        for paper in &all_papers {
            // add paper ID, title and authors to the as a comment
            survey_text.push_str(&format!(
                "% Paper ID: {}\n% Title: {}\n% Authors: {}\n\n",
                paper.id, paper.title, paper.authors
            ));
            for section in &paper.sections {
                survey_text.push_str(&format!("\\section{{{}}}\n\n", section.title));
                let (normalized_content, _) =
                    consolidated_bibliography.normalize_citations(&section.content)?;
                survey_text.push_str(&normalized_content);
                survey_text.push_str("\n\n");
            }
        }

        // Generate the final BibTeX string.
        let bibtex = format_bibliography_as_bibtex(&consolidated_bibliography);

        Ok((survey_text, bibtex))
    }

    /// Convert a Bibliography object to a proper BibTeX-formatted string.
    pub fn format_bibliography_as_bibtex(bibliography: &latex::Bibliography) -> String {
        let mut bibtex = String::new();
        let mut keys: Vec<_> = bibliography.iter().map(|entry| entry.key.clone()).collect();
        keys.sort(); // Sort for consistent output.

        for key in keys {
            if let Some(entry) = bibliography.get(&key) {
                let normalized_key = bibliography.normalize_citation_key(entry);
                bibtex.push_str(&format!("@{}{{{},\n", entry.entry_type, normalized_key));

                let mut fields: Vec<_> = entry.fields.keys().filter(|&k| k != "raw").cloned().collect();
                fields.sort(); // Sort for consistent output.
                for field in fields {
                    if let Some(value) = entry.fields.get(&field) {
                        // remove curly braces from the value
                        let clean_value = value.replace("{", "").replace("}", "");
                        bibtex.push_str(&format!("  {} = {{{}}},\n", field, clean_value));
                    }
                }
                bibtex.push_str("}\n\n");
            }
        }
        bibtex
    }
}
