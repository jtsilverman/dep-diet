use std::fs;
use std::process::Command;

fn cargo_bin() -> std::path::PathBuf {
    // Build first, then locate the binary
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to build");
    assert!(output.status.success(), "cargo build failed: {}", String::from_utf8_lossy(&output.stderr));

    let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/dep-diet");
    assert!(target_dir.exists(), "binary not found at {:?}", target_dir);
    target_dir
}

fn setup_fixture(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("dep-diet-integration-{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_no_package_json_exits_with_error() {
    let dir = setup_fixture("no-pkg");
    let output = Command::new(cargo_bin())
        .arg(dir.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No package.json found"), "expected error about missing package.json, got: {}", stderr);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_empty_dependencies() {
    let dir = setup_fixture("empty-deps");
    fs::write(dir.join("package.json"), r#"{"name": "test", "version": "1.0.0"}"#).unwrap();

    let output = Command::new(cargo_bin())
        .arg(dir.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No dependencies found"), "expected 'No dependencies found', got: {}", stdout);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_json_output_parses() {
    let dir = setup_fixture("json-output");
    // Use a package that definitely exists on npm
    fs::write(dir.join("package.json"), r#"{"dependencies": {"is-number": "^7.0.0"}}"#).unwrap();

    let output = Command::new(cargo_bin())
        .args([dir.to_str().unwrap(), "--json"])
        .output()
        .expect("failed to run");

    assert!(output.status.success(), "command failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("failed to parse JSON output: {}. Output was: {}", e, stdout));

    assert!(parsed.get("total_packages").is_some());
    assert!(parsed.get("packages").is_some());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_unused_flag_reports_unused_deps() {
    let dir = setup_fixture("unused-flag");
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(dir.join("package.json"), r#"{"dependencies": {"is-number": "^7.0.0"}}"#).unwrap();
    // No source files import is-number, so it should be flagged as unused
    fs::write(src.join("index.js"), "console.log('hello');").unwrap();

    let output = Command::new(cargo_bin())
        .args([dir.to_str().unwrap(), "--unused", "--json"])
        .output()
        .expect("failed to run");

    assert!(output.status.success(), "command failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("failed to parse JSON: {}. Output: {}", e, stdout));

    let unused = parsed.get("unused").and_then(|u| u.as_array()).expect("missing unused field");
    assert!(unused.iter().any(|u| u.as_str() == Some("is-number")),
        "is-number should be reported as unused, got: {:?}", unused);

    fs::remove_dir_all(&dir).ok();
}
