use bibextract::latex::citation::{related_work_section, extract_sections_from_latex};
use bibextract::latex::Bibliography;

#[test]
fn test_related_work_section() {
    assert!(related_work_section("related work"));
    assert!(related_work_section("Background"));
    assert!(related_work_section("Literature Review"));
    assert!(related_work_section("prior work"));
    assert!(related_work_section("previous work"));
    assert!(related_work_section("state of the art"));
    assert!(related_work_section("comparative analysis"));
    assert!(related_work_section("context"));
    assert!(related_work_section("existing work"));
    assert!(related_work_section("existing approaches"));
    assert!(related_work_section("existing methods"));
    assert!(related_work_section("review of the literature"));
    assert!(related_work_section("previous approaches"));
    assert!(related_work_section("foundation"));
    assert!(!related_work_section("introduction"));
    assert!(!related_work_section("conclusion"));
}

#[test]
fn test_extract_sections_from_latex() {
    let bibliography = Bibliography::new();

    // Test with no sections
    let content = "Some random text.";
    let sections = extract_sections_from_latex(content, &bibliography).unwrap();
    assert!(sections.is_empty());

    // Test with a related work section
    let content = r"\section{Related Work}
This is the content of related work. \cite{citation1}";
    let sections = extract_sections_from_latex(content, &bibliography).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "Related Work");
    assert!(sections[0].content.contains("This is the content"));
    assert_eq!(sections[0].citations, vec!["citation1"]);

    // Test with a non-related work section
    let content = r"\section{Introduction}
This is the introduction.";
    let sections = extract_sections_from_latex(content, &bibliography).unwrap();
    assert!(sections.is_empty());

    // Test with multiple sections, some related, some not
    let content = r"\section{Introduction}
Intro content.
\section{Background}
Background content. \cite{citation2}
\section{Methodology}
Methodology content.
\section{Previous Work}
Previous work content. \cite{citation3}";
    let sections = extract_sections_from_latex(content, &bibliography).unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].title, "Background");
    assert_eq!(sections[0].citations, vec!["citation2"]);
    assert_eq!(sections[1].title, "Previous Work");
    assert_eq!(sections[1].citations, vec!["citation3"]);

    // Test with subsections
    let content = r"\section{Main Section}
Main content.
\subsection{Related Work}
Subsection content. \cite{citation4}";
    let sections = extract_sections_from_latex(content, &bibliography).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "Related Work");
    assert_eq!(sections[0].citations, vec!["citation4"]);

    // Test with multiple citations in one section
    let content = r"\section{Related Work}
Content with \cite{a, b} and \cite{c}.";
    let sections = extract_sections_from_latex(content, &bibliography).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].citations, vec!["a", "b", "c"]);

    // Test with no citations in a related work section
    let content = r"\section{Background}
Content without citations.";
    let sections = extract_sections_from_latex(content, &bibliography).unwrap();
    assert_eq!(sections.len(), 1);
    assert!(sections[0].citations.is_empty());
}
