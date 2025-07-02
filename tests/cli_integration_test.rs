use assert_cmd::Command;

#[test]
fn test_cli_invalid_paper_id() {
    let mut cmd = Command::cargo_bin("bibextract").unwrap();
    cmd.arg("-p").arg("invalid_id");
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("API error: Failed to download source: HTTP 404 Not Found"));
}
