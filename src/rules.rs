use std::path::Path;

use log::warn;

/// Describes a build artifact directory and how to identify it.
#[derive(Debug, Clone)]
pub struct ArtifactRule {
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

/// Returns the full set of artifact rules, ordered so that more specific markers
/// come first (helps with disambiguation of `target/`, `build/`, etc.).
pub fn all_rules() -> Vec<MatchableRule> {
    vec![
        // Java/Maven
        mr("Java/Maven", "target", &["pom.xml"]),
        // Rust/Cargo
        mr("Rust/Cargo", "target", &["Cargo.toml"]),
        // Scala/SBT
        mr("Scala/SBT", "target", &["build.sbt"]),
        // Node.js
        mr("Node.js", "node_modules", &["package.json"]),
        mr("Node.js", ".next", &["package.json"]),
        mr("Node.js", ".nuxt", &["package.json"]),
        mr("Node.js", ".output", &["package.json"]),
        // Swift/SPM
        mr("Swift/SPM", ".build", &["Package.swift"]),
        // Python -- no-marker variants
        MatchableRule {
            rule: ArtifactRule {
                build_system: "Python",
                artifact_dir: "__pycache__",
                marker: MarkerKind::Always,
            },
            dir_match: DirMatch::Exact("__pycache__"),
        },
        MatchableRule {
            rule: ArtifactRule {
                build_system: "Python",
                artifact_dir: ".mypy_cache",
                marker: MarkerKind::Always,
            },
            dir_match: DirMatch::Exact(".mypy_cache"),
        },
        MatchableRule {
            rule: ArtifactRule {
                build_system: "Python",
                artifact_dir: ".pytest_cache",
                marker: MarkerKind::Always,
            },
            dir_match: DirMatch::Exact(".pytest_cache"),
        },
        // Python -- marker variants
        mr_multi(
            "Python",
            ".venv",
            &["pyproject.toml", "setup.py", "requirements.txt"],
        ),
        mr_multi(
            "Python",
            "venv",
            &["pyproject.toml", "setup.py", "requirements.txt"],
        ),
        mr_multi(
            "Python",
            ".tox",
            &["pyproject.toml", "setup.py", "requirements.txt"],
        ),
        // Python egg-info (suffix match)
        MatchableRule {
            rule: ArtifactRule {
                build_system: "Python",
                artifact_dir: "*.egg-info",
                marker: MarkerKind::Files(&["pyproject.toml", "setup.py", "requirements.txt"]),
            },
            dir_match: DirMatch::Suffix(".egg-info"),
        },
        // Android/Gradle
        mr_multi(
            "Android/Gradle",
            "build",
            &["build.gradle", "build.gradle.kts"],
        ),
        mr_multi(
            "Android/Gradle",
            ".gradle",
            &["build.gradle", "build.gradle.kts"],
        ),
        // C/C++/CMake
        mr("C/C++/CMake", "build", &["CMakeLists.txt"]),
        mr("C/C++/CMake", "CMakeFiles", &["CMakeLists.txt"]),
        // .NET/C#
        MatchableRule {
            rule: ArtifactRule {
                build_system: ".NET/C#",
                artifact_dir: "bin",
                marker: MarkerKind::GlobSuffix(".csproj"),
            },
            dir_match: DirMatch::Exact("bin"),
        },
        MatchableRule {
            rule: ArtifactRule {
                build_system: ".NET/C#",
                artifact_dir: "obj",
                marker: MarkerKind::GlobSuffix(".csproj"),
            },
            dir_match: DirMatch::Exact("obj"),
        },
        // .NET/C# -- .sln marker
        MatchableRule {
            rule: ArtifactRule {
                build_system: ".NET/C#",
                artifact_dir: "bin",
                marker: MarkerKind::GlobSuffix(".sln"),
            },
            dir_match: DirMatch::Exact("bin"),
        },
        MatchableRule {
            rule: ArtifactRule {
                build_system: ".NET/C#",
                artifact_dir: "obj",
                marker: MarkerKind::GlobSuffix(".sln"),
            },
            dir_match: DirMatch::Exact("obj"),
        },
        // Elixir/Mix
        mr("Elixir/Mix", "_build", &["mix.exs"]),
        mr("Elixir/Mix", "deps", &["mix.exs"]),
        // Haskell/Stack
        mr("Haskell/Stack", ".stack-work", &["stack.yaml"]),
        // Haskell/Cabal
        MatchableRule {
            rule: ArtifactRule {
                build_system: "Haskell/Cabal",
                artifact_dir: "dist-newstyle",
                marker: MarkerKind::GlobSuffix(".cabal"),
            },
            dir_match: DirMatch::Exact("dist-newstyle"),
        },
        // Dart/Flutter
        mr("Dart/Flutter", ".dart_tool", &["pubspec.yaml"]),
        mr("Dart/Flutter", "build", &["pubspec.yaml"]),
        // Zig
        mr("Zig", "zig-out", &["build.zig"]),
        mr("Zig", "zig-cache", &["build.zig"]),
        // PHP/Composer
        mr("PHP/Composer", "vendor", &["composer.json"]),
        // CocoaPods
        mr("CocoaPods", "Pods", &["Podfile"]),
        // Ruby/Bundler -- special: matches `bundle` inside a `vendor/` directory.
        // The scanner checks the grandparent for `Gemfile`.
        mr("Ruby/Bundler", "bundle", &["Gemfile"]),
    ]
}

/// Shorthand for an exact-match rule with a single-file marker set.
fn mr(
    build_system: &'static str,
    artifact_dir: &'static str,
    markers: &'static [&'static str],
) -> MatchableRule {
    MatchableRule {
        rule: ArtifactRule {
            build_system,
            artifact_dir,
            marker: MarkerKind::Files(markers),
        },
        dir_match: DirMatch::Exact(artifact_dir),
    }
}

/// Shorthand for an exact-match rule with multiple marker files.
fn mr_multi(
    build_system: &'static str,
    artifact_dir: &'static str,
    markers: &'static [&'static str],
) -> MatchableRule {
    mr(build_system, artifact_dir, markers)
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn all_rules_are_non_empty() {
        let rules = all_rules();
        assert!(!rules.is_empty());
        for r in &rules {
            assert!(!r.rule.build_system.is_empty());
            assert!(!r.rule.artifact_dir.is_empty());
        }
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
