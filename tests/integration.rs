use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("clean-builds").unwrap()
}

fn set_up_rust_project(tmp: &TempDir) {
    let project = tmp.path().join("my-rust-app");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname = \"app\"").unwrap();
    let target = project.join("target");
    fs::create_dir_all(target.join("debug")).unwrap();
    fs::write(target.join("debug").join("app"), "binary data").unwrap();
}

fn set_up_node_project(tmp: &TempDir) {
    let project = tmp.path().join("my-node-app");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("package.json"), "{}").unwrap();
    let nm = project.join("node_modules");
    fs::create_dir_all(nm.join("lodash")).unwrap();
    fs::write(nm.join("lodash").join("index.js"), "module.exports = {}").unwrap();
}

#[test]
fn dry_run_shows_summary() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);

    cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust/Cargo"))
        .stdout(predicate::str::contains("Run with --delete"));
}

#[test]
fn dry_run_does_not_delete() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);

    cmd().arg(tmp.path()).assert().success();

    // target/ should still exist
    assert!(tmp.path().join("my-rust-app").join("target").exists());
}

#[test]
fn delete_with_yes_removes_artifacts() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);

    cmd()
        .arg(tmp.path())
        .arg("--delete")
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 1 of 1"));

    assert!(!tmp.path().join("my-rust-app").join("target").exists());
    // Cargo.toml should still exist
    assert!(tmp.path().join("my-rust-app").join("Cargo.toml").exists());
}

#[test]
fn verbose_shows_paths() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);

    cmd()
        .arg(tmp.path())
        .arg("--verbose")
        .assert()
        .success()
        .stdout(predicate::str::contains("target"));
}

#[test]
fn multiple_build_systems() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);
    set_up_node_project(&tmp);

    cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust/Cargo"))
        .stdout(predicate::str::contains("Node.js"))
        .stdout(predicate::str::contains("Total"));
}

#[test]
fn exclude_by_glob_pattern() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);

    cmd()
        .arg(tmp.path())
        .arg("--exclude")
        .arg("my-rust*")
        .assert()
        .success()
        .stdout(predicate::str::contains("No build artifacts found."));
}

#[test]
fn include_only_matching_artifacts() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);
    set_up_node_project(&tmp);

    cmd()
        .arg(tmp.path())
        .arg("--include")
        .arg("node_modules")
        .assert()
        .success()
        .stdout(predicate::str::contains("Node.js"))
        .stdout(predicate::str::contains("Rust/Cargo").not());
}

#[test]
fn include_and_exclude_combined() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);
    set_up_node_project(&tmp);

    // Include both, but exclude the node one by project dir prefix
    cmd()
        .arg(tmp.path())
        .arg("--include")
        .arg("target")
        .arg("--include")
        .arg("node_modules")
        .arg("--exclude")
        .arg("my-node*")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust/Cargo"))
        .stdout(predicate::str::contains("Node.js").not());
}

#[test]
fn multiple_include_patterns() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);
    set_up_node_project(&tmp);

    cmd()
        .arg(tmp.path())
        .arg("--include")
        .arg("node_modules")
        .arg("--include")
        .arg("target")
        .assert()
        .success()
        .stdout(predicate::str::contains("Node.js"))
        .stdout(predicate::str::contains("Rust/Cargo"));
}

#[test]
fn exclude_by_directory_name_prefix() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);
    set_up_node_project(&tmp);

    // Exclude projects starting with "my-"
    cmd()
        .arg(tmp.path())
        .arg("--exclude")
        .arg("my-*")
        .assert()
        .success()
        .stdout(predicate::str::contains("No build artifacts found."));
}

#[test]
fn invalid_pattern_exits_with_error() {
    let tmp = TempDir::new().unwrap();

    cmd()
        .arg(tmp.path())
        .arg("--exclude")
        .arg("[invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn no_artifacts_found() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("empty-project")).unwrap();

    cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No build artifacts found."));
}

#[test]
fn nonexistent_path() {
    cmd()
        .arg("/nonexistent/path/that/does/not/exist")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn delete_multiple_systems_with_yes() {
    let tmp = TempDir::new().unwrap();
    set_up_rust_project(&tmp);
    set_up_node_project(&tmp);

    cmd()
        .arg(tmp.path())
        .arg("--delete")
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 2 of 2"));

    assert!(!tmp.path().join("my-rust-app").join("target").exists());
    assert!(!tmp.path().join("my-node-app").join("node_modules").exists());
}

#[test]
fn help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recursively scan"))
        .stdout(predicate::str::contains("--delete"));
}

#[test]
fn pycache_detected_without_marker() {
    let tmp = TempDir::new().unwrap();
    let pycache = tmp.path().join("src").join("__pycache__");
    fs::create_dir_all(&pycache).unwrap();
    fs::write(pycache.join("module.cpython-312.pyc"), "bytecode").unwrap();

    cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Python"));
}
