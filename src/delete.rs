use std::io::{BufRead, Write};
use std::path::Path;

use rayon::prelude::*;

use crate::scanner::Artifact;
use crate::size::format_size;

/// Error type for deletion operations.
#[derive(thiserror::Error, Debug)]
pub enum DeleteError {
    #[error("failed to delete {path}: {source}")]
    RemoveDir {
        path: String,
        source: std::io::Error,
    },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Prompt the user for confirmation and delete artifacts in parallel if confirmed.
/// Returns the number of artifacts deleted, or 0 if the user declined.
pub fn confirm_and_delete(
    out: &mut dyn Write,
    input: &mut dyn BufRead,
    artifacts: &[Artifact],
    skip_confirm: bool,
) -> Result<usize, DeleteError> {
    let total_bytes: u64 = artifacts.iter().map(|a| a.size_bytes).sum();

    if !skip_confirm {
        write!(
            out,
            "\nDelete {} targets ({})? [y/N] ",
            artifacts.len(),
            format_size(total_bytes)
        )?;
        out.flush()?;

        let mut response = String::new();
        input.read_line(&mut response)?;
        let response = response.trim().to_lowercase();
        if response != "y" && response != "yes" {
            writeln!(out, "Aborted.")?;
            return Ok(0);
        }
    }

    let results: Vec<Result<(), DeleteError>> = artifacts
        .par_iter()
        .map(|artifact| delete_artifact(&artifact.path))
        .collect();

    let mut deleted = 0;
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(()) => deleted += 1,
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        writeln!(out)?;
        for err in &errors {
            writeln!(out, "Error: {err}")?;
        }
    }

    writeln!(
        out,
        "\nDeleted {deleted} of {} artifact directories ({}).",
        artifacts.len(),
        format_size(total_bytes)
    )?;

    Ok(deleted)
}

/// Delete a single artifact directory.
fn delete_artifact(path: &Path) -> Result<(), DeleteError> {
    std::fs::remove_dir_all(path).map_err(|e| DeleteError::RemoveDir {
        path: path.display().to_string(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use tempfile::TempDir;

    fn make_test_artifact(tmp: &TempDir, name: &'static str) -> Artifact {
        let path = tmp.path().join(name);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("file.txt"), "test data").unwrap();
        Artifact {
            path,
            build_system: "Test",
            artifact_dir: name,
            size_bytes: 9,
        }
    }

    #[test]
    fn confirm_yes_deletes() {
        let tmp = TempDir::new().unwrap();
        let artifacts = vec![make_test_artifact(&tmp, "target")];

        let mut out = Vec::new();
        let mut input = Cursor::new(b"y\n".to_vec());
        let deleted = confirm_and_delete(&mut out, &mut input, &artifacts, false).unwrap();

        assert_eq!(deleted, 1);
        assert!(!tmp.path().join("target").exists());
    }

    #[test]
    fn confirm_no_aborts() {
        let tmp = TempDir::new().unwrap();
        let artifacts = vec![make_test_artifact(&tmp, "target")];

        let mut out = Vec::new();
        let mut input = Cursor::new(b"n\n".to_vec());
        let deleted = confirm_and_delete(&mut out, &mut input, &artifacts, false).unwrap();

        assert_eq!(deleted, 0);
        assert!(tmp.path().join("target").exists());
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Aborted"));
    }

    #[test]
    fn skip_confirm_deletes_immediately() {
        let tmp = TempDir::new().unwrap();
        let artifacts = vec![make_test_artifact(&tmp, "build")];

        let mut out = Vec::new();
        let mut input = Cursor::new(Vec::new());
        let deleted = confirm_and_delete(&mut out, &mut input, &artifacts, true).unwrap();

        assert_eq!(deleted, 1);
        assert!(!tmp.path().join("build").exists());
    }

    #[test]
    fn empty_response_aborts() {
        let tmp = TempDir::new().unwrap();
        let artifacts = vec![make_test_artifact(&tmp, "target")];

        let mut out = Vec::new();
        let mut input = Cursor::new(b"\n".to_vec());
        let deleted = confirm_and_delete(&mut out, &mut input, &artifacts, false).unwrap();

        assert_eq!(deleted, 0);
        assert!(tmp.path().join("target").exists());
    }

    #[test]
    fn confirm_yes_full_word() {
        let tmp = TempDir::new().unwrap();
        let artifacts = vec![make_test_artifact(&tmp, "node_modules")];

        let mut out = Vec::new();
        let mut input = Cursor::new(b"yes\n".to_vec());
        let deleted = confirm_and_delete(&mut out, &mut input, &artifacts, false).unwrap();

        assert_eq!(deleted, 1);
    }

    #[test]
    fn deletes_multiple_artifacts() {
        let tmp = TempDir::new().unwrap();
        let artifacts = vec![
            make_test_artifact(&tmp, "target"),
            make_test_artifact(&tmp, "build"),
            make_test_artifact(&tmp, "node_modules"),
        ];

        let mut out = Vec::new();
        let mut input = Cursor::new(Vec::new());
        let deleted = confirm_and_delete(&mut out, &mut input, &artifacts, true).unwrap();

        assert_eq!(deleted, 3);
        assert!(!tmp.path().join("target").exists());
        assert!(!tmp.path().join("build").exists());
        assert!(!tmp.path().join("node_modules").exists());
    }

    #[test]
    fn output_includes_summary() {
        let tmp = TempDir::new().unwrap();
        let artifacts = vec![make_test_artifact(&tmp, "target")];

        let mut out = Vec::new();
        let mut input = Cursor::new(Vec::new());
        confirm_and_delete(&mut out, &mut input, &artifacts, true).unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Deleted 1 of 1"));
    }
}
