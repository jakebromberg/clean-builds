use std::path::Path;

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::scanner::Artifact;

/// Compute directory sizes for all artifacts in parallel.
pub fn compute_sizes(artifacts: &mut [Artifact]) {
    let sizes: Vec<u64> = artifacts.par_iter().map(|a| dir_size(&a.path)).collect();

    for (artifact, size) in artifacts.iter_mut().zip(sizes) {
        artifact.size_bytes = size;
    }
}

/// Calculate the total size of a directory tree.
fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

/// Format a byte count as a human-readable string.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kb() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
    }

    #[test]
    fn format_size_mb() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(10 * 1024 * 1024 + 512 * 1024), "10.5 MB");
    }

    #[test]
    fn format_size_gb() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(
            format_size(2 * 1024 * 1024 * 1024 + 512 * 1024 * 1024),
            "2.5 GB"
        );
    }

    #[test]
    fn compute_sizes_populates_artifacts() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("target");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file.o"), "hello world").unwrap(); // 11 bytes

        let mut artifacts = vec![Artifact {
            path: dir.clone(),
            build_system: "Rust/Cargo",
            artifact_dir: "target",
            size_bytes: 0,
        }];

        compute_sizes(&mut artifacts);
        assert_eq!(artifacts[0].size_bytes, 11);
    }
}
