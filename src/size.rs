use std::path::Path;

use jwalk::{Parallelism, WalkDir};
use log::debug;
use rayon::prelude::*;

use crate::scanner::Artifact;

/// Compute directory sizes for all artifacts in parallel.
pub fn compute_sizes(artifacts: &mut [Artifact]) {
    let sizes: Vec<u64> = artifacts
        .par_iter()
        .map(|a| {
            let size = dir_size(&a.path);
            debug!("{}: {}", a.path.display(), format_size(size));
            size
        })
        .collect();

    for (artifact, size) in artifacts.iter_mut().zip(sizes) {
        artifact.size_bytes = size;
    }
}

/// Calculate the total size of a directory tree.
///
/// Uses serial walking to avoid contention with the outer rayon `par_iter`
/// that drives `compute_sizes`. Both share rayon's global thread pool, and
/// nested parallel walks deadlock when the pool is saturated.
fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .parallelism(Parallelism::Serial)
        .follow_links(false)
        .skip_hidden(false)
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

    /// Reproduces thread-pool contention between rayon par_iter and jwalk.
    /// With enough artifacts saturating the rayon global pool, jwalk's
    /// internal parallel walkers can't make progress and return 0.
    #[test]
    fn compute_sizes_many_artifacts_no_zeros() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let num_threads = rayon::current_num_threads();
        // Create more artifacts than rayon threads to trigger contention.
        let count = num_threads * 4;

        let mut artifacts: Vec<Artifact> = (0..count)
            .map(|i| {
                let dir = tmp.path().join(format!("project-{i}/node_modules"));
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join("file.js"), "content").unwrap();
                Artifact {
                    path: dir,
                    build_system: "Node.js",
                    artifact_dir: "node_modules",
                    size_bytes: 0,
                }
            })
            .collect();

        compute_sizes(&mut artifacts);

        let zeros: Vec<_> = artifacts
            .iter()
            .filter(|a| a.size_bytes == 0)
            .map(|a| a.path.display().to_string())
            .collect();
        assert!(
            zeros.is_empty(),
            "{} of {} artifacts reported 0 bytes: {:?}",
            zeros.len(),
            count,
            &zeros[..zeros.len().min(5)]
        );
    }
}
