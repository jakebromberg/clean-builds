use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::rules::{MatchableRule, all_rules, has_marker, matches_dir};

/// A detected build artifact.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: PathBuf,
    pub build_system: &'static str,
    pub artifact_dir: &'static str,
    /// Computed later by `size.rs`.
    pub size_bytes: u64,
}

/// Scan `root` for build artifacts, skipping `.git` directories.
pub fn scan(root: &Path) -> Vec<Artifact> {
    let rules = all_rules();
    let mut artifacts = Vec::new();
    // Track paths we've already recorded as artifacts so we don't descend into them.
    let mut pruned: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            // Only filter directories.
            if !entry.file_type().is_dir() {
                return true;
            }
            let name = entry.file_name().to_string_lossy();

            // Always skip .git
            if name == ".git" {
                return false;
            }

            true
        });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path();

        // If this path is under an already-pruned artifact, skip it.
        if pruned.iter().any(|p| path.starts_with(p)) {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if let Some(artifact) = try_match(path, dir_name, &rules) {
            pruned.push(path.to_path_buf());
            artifacts.push(artifact);
        }
    }

    artifacts
}

/// Try to match a directory against all rules. Returns the first match.
fn try_match(path: &Path, dir_name: &str, rules: &[MatchableRule]) -> Option<Artifact> {
    let parent = path.parent()?;

    for mr in rules {
        if !matches_dir(dir_name, &mr.dir_match) {
            continue;
        }

        // Special case: Ruby/Bundler requires parent to be named "vendor"
        // and marker (Gemfile) in grandparent.
        if mr.rule.build_system == "Ruby/Bundler" {
            let parent_name = parent.file_name().and_then(|n| n.to_str());
            if parent_name != Some("vendor") {
                continue;
            }
            let grandparent = parent.parent()?;
            if !has_marker(grandparent, &mr.rule.marker) {
                continue;
            }
            return Some(Artifact {
                path: path.to_path_buf(),
                build_system: mr.rule.build_system,
                artifact_dir: mr.rule.artifact_dir,
                size_bytes: 0,
            });
        }

        // Normal case: check marker in parent directory.
        if has_marker(parent, &mr.rule.marker) {
            return Some(Artifact {
                path: path.to_path_buf(),
                build_system: mr.rule.build_system,
                artifact_dir: mr.rule.artifact_dir,
                size_bytes: 0,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use tempfile::TempDir;

    fn set_up_project(tmp: &TempDir, marker: &str, artifact_dir: &str) -> PathBuf {
        let project = tmp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join(marker), "").unwrap();
        let artifact = project.join(artifact_dir);
        fs::create_dir_all(&artifact).unwrap();
        // Put a file inside to make it non-empty
        fs::write(artifact.join("some_file"), "data").unwrap();
        project
    }

    #[test]
    fn detects_rust_target() {
        let tmp = TempDir::new().unwrap();
        set_up_project(&tmp, "Cargo.toml", "target");
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Rust/Cargo");
        assert_eq!(artifacts[0].artifact_dir, "target");
    }

    #[test]
    fn detects_node_modules() {
        let tmp = TempDir::new().unwrap();
        set_up_project(&tmp, "package.json", "node_modules");
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Node.js");
    }

    #[test]
    fn detects_maven_target() {
        let tmp = TempDir::new().unwrap();
        set_up_project(&tmp, "pom.xml", "target");
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Java/Maven");
    }

    #[test]
    fn detects_python_pycache_without_marker() {
        let tmp = TempDir::new().unwrap();
        let pycache = tmp.path().join("some_dir").join("__pycache__");
        fs::create_dir_all(&pycache).unwrap();
        fs::write(pycache.join("module.pyc"), "").unwrap();
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Python");
        assert_eq!(artifacts[0].artifact_dir, "__pycache__");
    }

    #[test]
    fn detects_python_venv_with_marker() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("pyproject");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("pyproject.toml"), "").unwrap();
        fs::create_dir_all(project.join(".venv")).unwrap();
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Python");
        assert_eq!(artifacts[0].artifact_dir, ".venv");
    }

    #[test]
    fn ignores_venv_without_marker() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("random");
        fs::create_dir_all(project.join(".venv")).unwrap();
        let artifacts = scan(tmp.path());
        assert!(artifacts.is_empty());
    }

    #[test]
    fn detects_gradle_build() {
        let tmp = TempDir::new().unwrap();
        set_up_project(&tmp, "build.gradle", "build");
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Android/Gradle");
    }

    #[test]
    fn detects_cmake_build() {
        let tmp = TempDir::new().unwrap();
        set_up_project(&tmp, "CMakeLists.txt", "build");
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "C/C++/CMake");
    }

    #[test]
    fn detects_dotnet_with_csproj() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("myapp");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("MyApp.csproj"), "").unwrap();
        fs::create_dir_all(project.join("bin")).unwrap();
        fs::create_dir_all(project.join("obj")).unwrap();
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 2);
        let systems: Vec<&str> = artifacts.iter().map(|a| a.build_system).collect();
        assert!(systems.iter().all(|s| *s == ".NET/C#"));
    }

    #[test]
    fn detects_egg_info_suffix() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("pylib");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("setup.py"), "").unwrap();
        fs::create_dir_all(project.join("mylib.egg-info")).unwrap();
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Python");
    }

    #[test]
    fn skips_git_directory() {
        let tmp = TempDir::new().unwrap();
        let git = tmp.path().join(".git");
        fs::create_dir_all(&git).unwrap();
        let artifacts = scan(tmp.path());
        assert!(artifacts.is_empty());
    }

    #[test]
    fn no_false_positive_on_generic_build_dir() {
        let tmp = TempDir::new().unwrap();
        // build/ without any marker files should not match
        let project = tmp.path().join("generic");
        fs::create_dir_all(project.join("build")).unwrap();
        let artifacts = scan(tmp.path());
        assert!(artifacts.is_empty());
    }

    #[test]
    fn prunes_nested_artifacts() {
        let tmp = TempDir::new().unwrap();
        // A node project with node_modules that contains a nested project
        let project = tmp.path().join("app");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package.json"), "").unwrap();
        let nm = project.join("node_modules");
        fs::create_dir_all(&nm).unwrap();
        // Nested package inside node_modules
        let nested = nm.join("some-pkg");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("package.json"), "").unwrap();
        fs::create_dir_all(nested.join("node_modules")).unwrap();

        let artifacts = scan(tmp.path());
        // Should only detect the outer node_modules, not the nested one
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].path, nm);
    }

    #[test]
    fn detects_multiple_projects() {
        let tmp = TempDir::new().unwrap();

        // Rust project
        let rust_proj = tmp.path().join("rust-proj");
        fs::create_dir_all(&rust_proj).unwrap();
        fs::write(rust_proj.join("Cargo.toml"), "").unwrap();
        fs::create_dir_all(rust_proj.join("target")).unwrap();

        // Node project
        let node_proj = tmp.path().join("node-proj");
        fs::create_dir_all(&node_proj).unwrap();
        fs::write(node_proj.join("package.json"), "").unwrap();
        fs::create_dir_all(node_proj.join("node_modules")).unwrap();

        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 2);
        let systems: HashSet<&str> = artifacts.iter().map(|a| a.build_system).collect();
        assert!(systems.contains("Rust/Cargo"));
        assert!(systems.contains("Node.js"));
    }

    #[test]
    fn detects_ruby_vendor_bundle() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("rails-app");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("Gemfile"), "").unwrap();
        fs::create_dir_all(project.join("vendor").join("bundle")).unwrap();
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Ruby/Bundler");
    }

    #[test]
    fn ignores_vendor_without_bundle() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("app");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("Gemfile"), "").unwrap();
        // Just vendor/ without bundle/ inside
        fs::create_dir_all(project.join("vendor")).unwrap();
        let artifacts = scan(tmp.path());
        assert!(artifacts.is_empty());
    }

    #[test]
    fn detects_swift_spm_build() {
        let tmp = TempDir::new().unwrap();
        set_up_project(&tmp, "Package.swift", ".build");
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "Swift/SPM");
    }

    #[test]
    fn detects_elixir_mix() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("elixir-app");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("mix.exs"), "").unwrap();
        fs::create_dir_all(project.join("_build")).unwrap();
        fs::create_dir_all(project.join("deps")).unwrap();
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 2);
        assert!(artifacts.iter().all(|a| a.build_system == "Elixir/Mix"));
    }

    #[test]
    fn detects_cocoapods() {
        let tmp = TempDir::new().unwrap();
        set_up_project(&tmp, "Podfile", "Pods");
        let artifacts = scan(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].build_system, "CocoaPods");
    }
}
