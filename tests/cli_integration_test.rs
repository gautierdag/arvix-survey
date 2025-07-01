use assert_cmd::Command;

#[test]
fn test_cli_no_output_path() {
    let mut cmd = Command::cargo_bin("bibextract").unwrap();
    cmd.arg("-p").arg("2104.08653"); // Use a dummy paper ID
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("--- survey.tex ---"))
        .stdout(predicates::str::contains("--- bibliography.bib ---"));
}
