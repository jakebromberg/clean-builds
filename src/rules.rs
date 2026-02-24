use std::collections::BTreeSet;
use std::path::Path;

use log::warn;

/// Describes a build artifact directory and how to identify it.
#[derive(Debug, Clone)]
pub struct ArtifactRule {
    /// Short, CLI-friendly identifier for the build system (e.g., "cargo", "node").
    pub id: &'static str,
    pub build_system: &'static str,
    pub artifact_dir: &'static str,
    pub marker: MarkerKind,
}

/// How to confirm that an artifact directory belongs to a known build system.
#[derive(Debug, Clone)]
pub enum MarkerKind {
    /// Parent directory must contain one of these exact filenames.
    Files(&'static [&'static str]),
    /// Parent directory must contain a file matching a glob suffix (e.g., `.csproj`).
    GlobSuffix(&'static str),
    /// No marker needed -- always matches (e.g., `__pycache__`).
    Always,
}

/// Whether the artifact directory name is an exact match or a suffix glob.
#[derive(Debug, Clone)]
pub enum DirMatch {
    Exact(&'static str),
    Suffix(&'static str),
}

/// A rule with its matching strategy.
#[derive(Debug, Clone)]
pub struct MatchableRule {
    pub rule: ArtifactRule,
    pub dir_match: DirMatch,
}

/// Error from invalid system IDs in `--system` or `--exclude-system`.
#[derive(thiserror::Error, Debug)]
#[error("unknown build system: {id}\nValid systems: {valid}")]
pub struct SystemFilterError {
    id: String,
    valid: String,
}

/// Returns the full set of artifact rules, ordered so that more specific markers
/// come first (helps with disambiguation of `target/`, `build/`, etc.).
pub fn all_rules() -> Vec<MatchableRule> {
    vec![
        // Java/Maven
        mr("maven", "Java/Maven", "target", &["pom.xml"]),
        // Rust/Cargo
        mr("cargo", "Rust/Cargo", "target", &["Cargo.toml"]),
        // Scala/SBT
        mr("sbt", "Scala/SBT", "target", &["build.sbt"]),
        // Node.js
        mr("node", "Node.js", "node_modules", &["package.json"]),
        mr("node", "Node.js", ".next", &["package.json"]),
        mr("node", "Node.js", ".nuxt", &["package.json"]),
        mr("node", "Node.js", ".output", &["package.json"]),
        // Swift/SPM
        mr("spm", "Swift/SPM", ".build", &["Package.swift"]),
        // Python -- no-marker variants
        MatchableRule {
            rule: ArtifactRule {
                id: "python",
                build_system: "Python",
                artifact_dir: "__pycache__",
                marker: MarkerKind::Always,
            },
            dir_match: DirMatch::Exact("__pycache__"),
        },
        MatchableRule {
            rule: ArtifactRule {
                id: "python",
                build_system: "Python",
                artifact_dir: ".mypy_cache",
                marker: MarkerKind::Always,
            },
            dir_match: DirMatch::Exact(".mypy_cache"),
        },
        MatchableRule {
            rule: ArtifactRule {
                id: "python",
                build_system: "Python",
                artifact_dir: ".pytest_cache",
                marker: MarkerKind::Always,
            },
            dir_match: DirMatch::Exact(".pytest_cache"),
        },
        // Python -- marker variants
        mr_multi(
            "python",
            "Python",
            ".venv",
            &["pyproject.toml", "setup.py", "requirements.txt"],
        ),
        mr_multi(
            "python",
            "Python",
            "venv",
            &["pyproject.toml", "setup.py", "requirements.txt"],
        ),
        mr_multi(
            "python",
            "Python",
            ".tox",
            &["pyproject.toml", "setup.py", "requirements.txt"],
        ),
        // Python egg-info (suffix match)
        MatchableRule {
            rule: ArtifactRule {
                id: "python",
                build_system: "Python",
                artifact_dir: "*.egg-info",
                marker: MarkerKind::Files(&["pyproject.toml", "setup.py", "requirements.txt"]),
            },
            dir_match: DirMatch::Suffix(".egg-info"),
        },
        // Android/Gradle
        mr_multi(
            "gradle",
            "Android/Gradle",
            "build",
            &["build.gradle", "build.gradle.kts"],
        ),
        mr_multi(
            "gradle",
            "Android/Gradle",
            ".gradle",
            &["build.gradle", "build.gradle.kts"],
        ),
        // C/C++/CMake
        mr("cmake", "C/C++/CMake", "build", &["CMakeLists.txt"]),
        mr("cmake", "C/C++/CMake", "CMakeFiles", &["CMakeLists.txt"]),
        // .NET/C#
        MatchableRule {
            rule: ArtifactRule {
                id: "dotnet",
                build_system: ".NET/C#",
                artifact_dir: "bin",
                marker: MarkerKind::GlobSuffix(".csproj"),
            },
            dir_match: DirMatch::Exact("bin"),
        },
        MatchableRule {
            rule: ArtifactRule {
                id: "dotnet",
                build_system: ".NET/C#",
                artifact_dir: "obj",
                marker: MarkerKind::GlobSuffix(".csproj"),
            },
            dir_match: DirMatch::Exact("obj"),
        },
        // .NET/C# -- .sln marker
        MatchableRule {
            rule: ArtifactRule {
                id: "dotnet",
                build_system: ".NET/C#",
                artifact_dir: "bin",
                marker: MarkerKind::GlobSuffix(".sln"),
            },
            dir_match: DirMatch::Exact("bin"),
        },
        MatchableRule {
            rule: ArtifactRule {
                id: "dotnet",
                build_system: ".NET/C#",
                artifact_dir: "obj",
                marker: MarkerKind::GlobSuffix(".sln"),
            },
            dir_match: DirMatch::Exact("obj"),
        },
        // Elixir/Mix
        mr("mix", "Elixir/Mix", "_build", &["mix.exs"]),
        mr("mix", "Elixir/Mix", "deps", &["mix.exs"]),
        // Haskell/Stack
        mr("stack", "Haskell/Stack", ".stack-work", &["stack.yaml"]),
        // Haskell/Cabal
        MatchableRule {
            rule: ArtifactRule {
                id: "cabal",
                build_system: "Haskell/Cabal",
                artifact_dir: "dist-newstyle",
                marker: MarkerKind::GlobSuffix(".cabal"),
            },
            dir_match: DirMatch::Exact("dist-newstyle"),
        },
        // Dart/Flutter
        mr("flutter", "Dart/Flutter", ".dart_tool", &["pubspec.yaml"]),
        mr("flutter", "Dart/Flutter", "build", &["pubspec.yaml"]),
        // Zig
        mr("zig", "Zig", "zig-out", &["build.zig"]),
        mr("zig", "Zig", "zig-cache", &["build.zig"]),
        // PHP/Composer
        mr("composer", "PHP/Composer", "vendor", &["composer.json"]),
        // CocoaPods
        mr("cocoapods", "CocoaPods", "Pods", &["Podfile"]),
        // Ruby/Bundler -- special: matches `bundle` inside a `vendor/` directory.
        // The scanner checks the grandparent for `Gemfile`.
        mr("bundler", "Ruby/Bundler", "bundle", &["Gemfile"]),
    ]
}

/// Returns sorted, deduplicated `(id, display_name)` pairs for all build systems.
pub fn system_ids() -> Vec<(&'static str, &'static str)> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for r in all_rules() {
        if seen.insert(r.rule.id) {
            result.push((r.rule.id, r.rule.build_system));
        }
    }
    result.sort_by_key(|(id, _)| *id);
    result
}

/// Filter rules by system ID include/exclude lists.
///
/// When both slices are empty, returns all rules unchanged.
/// IDs are matched case-insensitively.
pub fn filter_rules_by_system(
    rules: Vec<MatchableRule>,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<MatchableRule>, SystemFilterError> {
    if include.is_empty() && exclude.is_empty() {
        return Ok(rules);
    }

    let valid_ids: BTreeSet<&str> = rules.iter().map(|r| r.rule.id).collect();

    // Validate and normalize IDs
    let normalize = |ids: &[String]| -> Result<Vec<String>, SystemFilterError> {
        ids.iter()
            .map(|id| {
                let lower = id.to_ascii_lowercase();
                if valid_ids.contains(lower.as_str()) {
                    Ok(lower)
                } else {
                    Err(SystemFilterError {
                        id: id.clone(),
                        valid: valid_ids.iter().copied().collect::<Vec<_>>().join(", "),
                    })
                }
            })
            .collect()
    };

    if !include.is_empty() {
        let normalized = normalize(include)?;
        Ok(rules
            .into_iter()
            .filter(|r| normalized.iter().any(|id| id == r.rule.id))
            .collect())
    } else {
        let normalized = normalize(exclude)?;
        Ok(rules
            .into_iter()
            .filter(|r| !normalized.iter().any(|id| id == r.rule.id))
            .collect())
    }
}

/// Shorthand for an exact-match rule with a single-file marker set.
fn mr(
    id: &'static str,
    build_system: &'static str,
    artifact_dir: &'static str,
    markers: &'static [&'static str],
) -> MatchableRule {
    MatchableRule {
        rule: ArtifactRule {
            id,
            build_system,
            artifact_dir,
            marker: MarkerKind::Files(markers),
        },
        dir_match: DirMatch::Exact(artifact_dir),
    }
}

/// Shorthand for an exact-match rule with multiple marker files.
fn mr_multi(
    id: &'static str,
    build_system: &'static str,
    artifact_dir: &'static str,
    markers: &'static [&'static str],
) -> MatchableRule {
    mr(id, build_system, artifact_dir, markers)
}

/// Check if a parent directory contains any file matching the given marker.
pub fn has_marker(parent: &Path, marker: &MarkerKind) -> bool {
    match marker {
        MarkerKind::Always => true,
        MarkerKind::Files(names) => names.iter().any(|name| parent.join(name).exists()),
        MarkerKind::GlobSuffix(suffix) => {
            let Ok(entries) = std::fs::read_dir(parent) else {
                warn!("Cannot read directory: {}", parent.display());
                return false;
            };
            entries.filter_map(|e| e.ok()).any(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|name| name.ends_with(suffix))
            })
        }
    }
}

/// Check if a directory name matches a rule's pattern.
pub fn matches_dir(dir_name: &str, dir_match: &DirMatch) -> bool {
    match dir_match {
        DirMatch::Exact(name) => dir_name == *name,
        DirMatch::Suffix(suffix) => dir_name.ends_with(suffix),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn all_rules_are_non_empty() {
        let rules = all_rules();
        assert!(!rules.is_empty());
        for r in &rules {
            assert!(!r.rule.id.is_empty());
            assert!(!r.rule.build_system.is_empty());
            assert!(!r.rule.artifact_dir.is_empty());
        }
    }

    #[test]
    fn all_ids_are_lowercase() {
        for r in all_rules() {
            assert_eq!(
                r.rule.id,
                r.rule.id.to_ascii_lowercase(),
                "ID '{}' is not lowercase",
                r.rule.id
            );
        }
    }

    #[test]
    fn system_ids_are_unique() {
        let ids = system_ids();
        let unique: HashSet<&str> = ids.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), unique.len(), "duplicate system IDs found");
    }

    #[test]
    fn system_ids_sorted() {
        let ids = system_ids();
        let sorted: Vec<_> = {
            let mut v = ids.clone();
            v.sort_by_key(|(id, _)| *id);
            v
        };
        assert_eq!(ids, sorted);
    }

    #[test]
    fn system_ids_covers_all_systems() {
        let ids = system_ids();
        let expected = [
            "bundler",
            "cabal",
            "cargo",
            "cmake",
            "cocoapods",
            "composer",
            "dotnet",
            "flutter",
            "gradle",
            "maven",
            "mix",
            "node",
            "python",
            "sbt",
            "spm",
            "stack",
            "zig",
        ];
        let actual: Vec<&str> = ids.iter().map(|(id, _)| *id).collect();
        for id in &expected {
            assert!(actual.contains(id), "Missing system ID: {id}");
        }
        assert_eq!(actual.len(), expected.len());
    }

    #[test]
    fn filter_include_keeps_matching() {
        let rules = all_rules();
        let filtered = filter_rules_by_system(rules, &["cargo".to_string()], &[]).unwrap();
        assert!(!filtered.is_empty());
        for r in &filtered {
            assert_eq!(r.rule.id, "cargo");
        }
    }

    #[test]
    fn filter_exclude_removes_matching() {
        let rules = all_rules();
        let total = rules.len();
        let filtered = filter_rules_by_system(rules, &[], &["python".to_string()]).unwrap();
        assert!(filtered.len() < total);
        for r in &filtered {
            assert_ne!(r.rule.id, "python");
        }
    }

    #[test]
    fn filter_empty_returns_all() {
        let rules = all_rules();
        let total = rules.len();
        let filtered = filter_rules_by_system(rules, &[], &[]).unwrap();
        assert_eq!(filtered.len(), total);
    }

    #[test]
    fn filter_unknown_id_errors() {
        let rules = all_rules();
        let result = filter_rules_by_system(rules, &["nonexistent".to_string()], &[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
        assert!(err.to_string().contains("cargo"));
    }

    #[test]
    fn filter_case_insensitive() {
        let rules = all_rules();
        let filtered = filter_rules_by_system(rules, &["CARGO".to_string()], &[]).unwrap();
        assert!(!filtered.is_empty());
        for r in &filtered {
            assert_eq!(r.rule.id, "cargo");
        }
    }

    #[test]
    fn filter_multiple_include() {
        let rules = all_rules();
        let filtered =
            filter_rules_by_system(rules, &["cargo".to_string(), "node".to_string()], &[]).unwrap();
        let ids: HashSet<&str> = filtered.iter().map(|r| r.rule.id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("cargo"));
        assert!(ids.contains("node"));
    }

    #[test]
    fn matches_dir_exact() {
        assert!(matches_dir(
            "node_modules",
            &DirMatch::Exact("node_modules")
        ));
        assert!(!matches_dir(
            "node_module",
            &DirMatch::Exact("node_modules")
        ));
    }

    #[test]
    fn matches_dir_suffix() {
        assert!(matches_dir("foo.egg-info", &DirMatch::Suffix(".egg-info")));
        assert!(!matches_dir("foo.egg", &DirMatch::Suffix(".egg-info")));
    }

    #[test]
    fn has_marker_always() {
        let tmp = TempDir::new().unwrap();
        assert!(has_marker(tmp.path(), &MarkerKind::Always));
    }

    #[test]
    fn has_marker_files_present() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        assert!(has_marker(tmp.path(), &MarkerKind::Files(&["Cargo.toml"])));
    }

    #[test]
    fn has_marker_files_absent() {
        let tmp = TempDir::new().unwrap();
        assert!(!has_marker(tmp.path(), &MarkerKind::Files(&["Cargo.toml"])));
    }

    #[test]
    fn has_marker_glob_suffix_present() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("MyProject.csproj"), "").unwrap();
        assert!(has_marker(tmp.path(), &MarkerKind::GlobSuffix(".csproj")));
    }

    #[test]
    fn has_marker_glob_suffix_absent() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("MyProject.txt"), "").unwrap();
        assert!(!has_marker(tmp.path(), &MarkerKind::GlobSuffix(".csproj")));
    }

    #[test]
    fn has_marker_multiple_files_any_match() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("setup.py"), "").unwrap();
        assert!(has_marker(
            tmp.path(),
            &MarkerKind::Files(&["pyproject.toml", "setup.py", "requirements.txt"])
        ));
    }

    #[test]
    fn rule_count_covers_all_build_systems() {
        let rules = all_rules();
        let systems: std::collections::HashSet<&str> =
            rules.iter().map(|r| r.rule.build_system).collect();
        // Verify we have all expected build systems
        let expected = [
            "Java/Maven",
            "Rust/Cargo",
            "Scala/SBT",
            "Node.js",
            "Swift/SPM",
            "Python",
            "Android/Gradle",
            "C/C++/CMake",
            ".NET/C#",
            "Elixir/Mix",
            "Haskell/Stack",
            "Haskell/Cabal",
            "Dart/Flutter",
            "Zig",
            "PHP/Composer",
            "CocoaPods",
            "Ruby/Bundler",
        ];
        for sys in &expected {
            assert!(systems.contains(sys), "Missing build system: {sys}");
        }
    }
}
