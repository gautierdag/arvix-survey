use bibextract::latex::parser::{find_bbl_files, find_main_tex_file, resolve_input_path, extract_all_latex_from_files};
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_find_bbl_files() {
    let dir = tempdir().unwrap();
    File::create(dir.path().join("test.bbl")).unwrap();
    File::create(dir.path().join("other.txt")).unwrap();
    let sub_dir = dir.path().join("sub");
    fs::create_dir(&sub_dir).unwrap();
    File::create(sub_dir.join("another.bbl")).unwrap();

    let bbl_files = find_bbl_files(dir.path()).unwrap();
    assert_eq!(bbl_files.len(), 2);
    assert!(bbl_files.iter().any(|p| p.ends_with("test.bbl")));
    assert!(bbl_files.iter().any(|p| p.ends_with("another.bbl")));
}

#[test]
fn test_resolve_input_path() {
    let dir = tempdir().unwrap();
    let tex_path = dir.path().join("included.tex");
    File::create(&tex_path).unwrap();

    // Test with extension
    let resolved_path = resolve_input_path(dir.path(), "included.tex").unwrap();
    assert_eq!(resolved_path, Some(tex_path.clone()));

    // Test without extension
    let resolved_path_no_ext = resolve_input_path(dir.path(), "included").unwrap();
    assert_eq!(resolved_path_no_ext, Some(tex_path.clone()));

    // Test non-existent file
    let non_existent_path = resolve_input_path(dir.path(), "nonexistent").unwrap();
    assert_eq!(non_existent_path, None);
}

#[test]
fn test_find_main_tex_file() {
    let dir = tempdir().unwrap();

    // Test with common name
    let main_tex_path = dir.path().join("main.tex");
    File::create(&main_tex_path).unwrap();
    assert_eq!(find_main_tex_file(dir.path()).unwrap(), main_tex_path);
    fs::remove_file(&main_tex_path).unwrap();

    // Test with \documentclass
    let article_tex_path = dir.path().join("article.tex");
    let mut article_file = File::create(&article_tex_path).unwrap();
    writeln!(article_file, r"\documentclass{{article}}").unwrap();
    assert_eq!(find_main_tex_file(dir.path()).unwrap(), article_tex_path);
    fs::remove_file(&article_tex_path).unwrap();

    // Test fallback to first .tex file
    let fallback_tex_path = dir.path().join("fallback.tex");
    File::create(&fallback_tex_path).unwrap();
    assert_eq!(find_main_tex_file(dir.path()).unwrap(), fallback_tex_path);
}

#[test]
fn test_extract_all_latex_from_files() {
    let dir = tempdir().unwrap();
    let main_path = dir.path().join("main.tex");
    let included_path = dir.path().join("included.tex");

    let mut main_file = File::create(&main_path).unwrap();
    writeln!(main_file, r"Main file content.
\input{{included}}").unwrap();

    let mut included_file = File::create(&included_path).unwrap();
    writeln!(included_file, "Included file content.").unwrap();

    let (full_content, included_files) = extract_all_latex_from_files(dir.path(), &main_path).unwrap();

    assert!(full_content.contains("Main file content."));
    assert!(full_content.contains("Included file content."));
    assert!(!full_content.contains(r"\input{included}"));
    assert_eq!(included_files.len(), 1);
    assert_eq!(included_files[0], included_path);
}