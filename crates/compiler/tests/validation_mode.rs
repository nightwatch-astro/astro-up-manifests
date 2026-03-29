use std::process::Command;

fn compiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_astro-up-compiler"))
}

#[test]
fn validate_valid_manifests() {
    let manifests_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");

    let output = compiler_bin()
        .arg("--manifests")
        .arg(&manifests_dir)
        .arg("--validate")
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code should be 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid"), "should report manifests as valid");
}

#[test]
fn validate_invalid_manifest_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("bad.toml"),
        "manifest_version = 1\nid = \"\"\nname = \"\"\ncategory = \"\"\ntype = \"\"\nslug = \"\"\n[install]\nmethod = \"unknown_method\"\n",
    )
    .unwrap();

    let output = compiler_bin()
        .arg("--manifests")
        .arg(dir.path())
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "exit code should be 2 for validation errors"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("error"), "should report errors");
}

#[test]
fn validate_nonexistent_dir_exits_1() {
    let output = compiler_bin()
        .arg("--manifests")
        .arg("/nonexistent/path")
        .arg("--validate")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "should fail for nonexistent directory"
    );
}
