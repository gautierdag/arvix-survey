use bibextract::latex::parser::{find_bbl_files, find_main_tex_file, extract_all_latex_from_files, resolve_input_path};
use bibextract::error::BibExtractError;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_find_bbl_files() {
    let dir = tempdir().unwrap();
    fs::File::create(dir.path().join("test1.bbl")).unwrap();
    fs::File::create(dir.path().join("test2.txt")).unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();
    fs::File::create(dir.path().join("subdir/test3.bbl")).unwrap();

    let bbl_files = find_bbl_files(dir.path()).unwrap();
    assert_eq!(bbl_files.len(), 2);
    assert!(bbl_files.contains(&dir.path().join("test1.bbl")));
    assert!(bbl_files.contains(&dir.path().join("subdir/test3.bbl")));
}


#[test]
fn test_find_main_tex_file() {
    let dir = tempdir().unwrap();

    // Test with common main file name
    fs::File::create(dir.path().join("main.tex")).unwrap();
    let main_tex = find_main_tex_file(dir.path()).unwrap();
    assert_eq!(main_tex, dir.path().join("main.tex"));
    fs::remove_file(dir.path().join("main.tex")).unwrap();

    // Test with \documentclass
    fs::write(dir.path().join("article.tex"), "\\documentclass{article}").unwrap();
    let main_tex = find_main_tex_file(dir.path()).unwrap();
    assert_eq!(main_tex, dir.path().join("article.tex"));
    fs::remove_file(dir.path().join("article.tex")).unwrap();

    // Test with no main file
    let result = find_main_tex_file(dir.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BibExtractError::ApiError(_)));
}

#[test]
fn test_resolve_input_path() {
    let dir = tempdir().unwrap();
    fs::File::create(dir.path().join("test.tex")).unwrap();

    // Test with direct path
    let path = resolve_input_path(dir.path(), "test.tex").unwrap().unwrap();
    assert_eq!(path, dir.path().join("test.tex"));

    // Test with missing .tex extension
    let path = resolve_input_path(dir.path(), "test").unwrap().unwrap();
    assert_eq!(path, dir.path().join("test.tex"));

    // Test with non-existent file
    let path = resolve_input_path(dir.path(), "nonexistent.tex").unwrap();
    assert!(path.is_none());
}

#[test]
fn test_extract_all_latex_from_files() {
    let dir = tempdir().unwrap();
    let main_tex_path = dir.path().join("main.tex");
    let included_tex_path = dir.path().join("included.tex");

    fs::write(&main_tex_path, "\\input{included}\nMain content.").unwrap();
    fs::write(&included_tex_path, "Included content.").unwrap();

    let (content, included_files) = extract_all_latex_from_files(dir.path(), &main_tex_path).unwrap();
    assert!(content.contains("Included content."));
    assert!(content.contains("Main content."));
    assert_eq!(included_files.len(), 1);
    assert_eq!(included_files[0], included_tex_path);
}
