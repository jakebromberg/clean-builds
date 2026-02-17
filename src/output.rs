use std::collections::BTreeMap;
use std::io::Write;

use crate::scanner::Artifact;
use crate::size::format_size;

/// Summary entry for a build system group.
struct GroupSummary {
    count: usize,
    total_bytes: u64,
}

/// Print a summary table of artifacts grouped by build system.
/// If `verbose`, also prints individual artifact paths.
pub fn print_summary(
    out: &mut dyn Write,
    artifacts: &[Artifact],
    verbose: bool,
) -> std::io::Result<()> {
    if artifacts.is_empty() {
        writeln!(out, "No build artifacts found.")?;
        return Ok(());
    }

    // Group by build system, preserving order with BTreeMap.
    let mut groups: BTreeMap<&str, GroupSummary> = BTreeMap::new();
    // Also collect paths per group for verbose mode.
    let mut paths_by_system: BTreeMap<&str, Vec<&Artifact>> = BTreeMap::new();

    for artifact in artifacts {
        let entry = groups.entry(artifact.build_system).or_insert(GroupSummary {
            count: 0,
            total_bytes: 0,
        });
        entry.count += 1;
        entry.total_bytes += artifact.size_bytes;

        if verbose {
            paths_by_system
                .entry(artifact.build_system)
                .or_default()
                .push(artifact);
        }
    }

    // Calculate column widths.
    let system_width = groups.keys().map(|k| k.len()).max().unwrap_or(12).max(12);
    let count_width = 5;
    let size_width = 10;

    // Header
    writeln!(
        out,
        "{:<system_width$}  {:>count_width$}  {:>size_width$}",
        "Build System", "Count", "Size"
    )?;
    writeln!(
        out,
        "{:<system_width$}  {:>count_width$}  {:>size_width$}",
        "-".repeat(system_width),
        "-".repeat(count_width),
        "-".repeat(size_width)
    )?;

    let mut total_count = 0;
    let mut total_bytes = 0u64;

    for (system, summary) in &groups {
        writeln!(
            out,
            "{:<system_width$}  {:>count_width$}  {:>size_width$}",
            system,
            summary.count,
            format_size(summary.total_bytes),
        )?;
        total_count += summary.count;
        total_bytes += summary.total_bytes;

        if verbose {
            if let Some(paths) = paths_by_system.get(system) {
                for artifact in paths {
                    writeln!(
                        out,
                        "  {} ({})",
                        artifact.path.display(),
                        format_size(artifact.size_bytes)
                    )?;
                }
            }
        }
    }

    // Total line
    writeln!(
        out,
        "{:<system_width$}  {:>count_width$}  {:>size_width$}",
        "-".repeat(system_width),
        "-".repeat(count_width),
        "-".repeat(size_width)
    )?;
    writeln!(
        out,
        "{:<system_width$}  {:>count_width$}  {:>size_width$}",
        "Total",
        total_count,
        format_size(total_bytes),
    )?;

    Ok(())
}

/// Print the dry-run footer message.
pub fn print_dry_run_footer(out: &mut dyn Write) -> std::io::Result<()> {
    writeln!(out)?;
    writeln!(out, "Run with --delete to remove these artifacts.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_artifact(system: &'static str, dir: &'static str, path: &str, size: u64) -> Artifact {
        Artifact {
            path: PathBuf::from(path),
            build_system: system,
            artifact_dir: dir,
            size_bytes: size,
        }
    }

    #[test]
    fn empty_artifacts() {
        let mut buf = Vec::new();
        print_summary(&mut buf, &[], false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("No build artifacts found."));
    }

    #[test]
    fn summary_table() {
        let artifacts = vec![
            make_artifact("Node.js", "node_modules", "/a/node_modules", 1024 * 1024),
            make_artifact(
                "Node.js",
                "node_modules",
                "/b/node_modules",
                2 * 1024 * 1024,
            ),
            make_artifact("Rust/Cargo", "target", "/c/target", 512 * 1024),
        ];
        let mut buf = Vec::new();
        print_summary(&mut buf, &artifacts, false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Node.js"));
        assert!(output.contains("Rust/Cargo"));
        assert!(output.contains("3.0 MB"));
        assert!(output.contains("Total"));
        assert!(output.contains("3"));
    }

    #[test]
    fn verbose_shows_paths() {
        let artifacts = vec![make_artifact(
            "Rust/Cargo",
            "target",
            "/projects/foo/target",
            1024,
        )];
        let mut buf = Vec::new();
        print_summary(&mut buf, &artifacts, true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("/projects/foo/target"));
    }

    #[test]
    fn dry_run_footer() {
        let mut buf = Vec::new();
        print_dry_run_footer(&mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Run with --delete"));
    }
}
