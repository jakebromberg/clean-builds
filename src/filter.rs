use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;

use crate::scanner::Artifact;

/// Error type for invalid filter patterns.
#[derive(thiserror::Error, Debug)]
#[error("invalid filter pattern: {0}")]
pub struct PatternError(#[from] globset::Error);

/// Filters artifacts by include/exclude glob patterns.
///
/// Patterns without `/` are auto-enhanced into two globs:
/// - `**/PATTERN`   — matches the pattern as a leaf path component
/// - `**/PATTERN/**` — matches the pattern as an ancestor component
///
/// Patterns containing `/` are used as-is.
///
/// Exclude takes precedence over include. If no includes are specified,
/// all artifacts are included.
pub struct ArtifactFilter {
    includes: Option<GlobSet>,
    excludes: GlobSet,
}

impl std::fmt::Debug for ArtifactFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArtifactFilter")
            .field("has_includes", &self.includes.is_some())
            .field("excludes_len", &self.excludes.len())
            .finish()
    }
}

impl ArtifactFilter {
    /// Build a filter from raw include and exclude pattern strings.
    pub fn new(
        include_patterns: &[String],
        exclude_patterns: &[String],
    ) -> Result<Self, PatternError> {
        let includes = if include_patterns.is_empty() {
            None
        } else {
            Some(build_glob_set(include_patterns)?)
        };

        let excludes = build_glob_set(exclude_patterns)?;

        Ok(Self { includes, excludes })
    }

    /// Test whether a single relative path matches the filter.
    pub fn matches(&self, relative_path: &Path) -> bool {
        if self.excludes.is_match(relative_path) {
            return false;
        }
        match &self.includes {
            None => true,
            Some(inc) => inc.is_match(relative_path),
        }
    }

    /// Filter a list of artifacts, matching their paths relative to `root`.
    pub fn apply(&self, root: &Path, artifacts: Vec<Artifact>) -> Vec<Artifact> {
        let before = artifacts.len();
        let filtered: Vec<Artifact> = artifacts
            .into_iter()
            .filter(|a| {
                let rel = a.path.strip_prefix(root).unwrap_or(&a.path);
                let matched = self.matches(rel);
                if !matched {
                    debug!("Filtered out: {}", rel.display());
                }
                matched
            })
            .collect();
        let removed = before - filtered.len();
        if removed > 0 {
            debug!("Filter: {} -> {} artifacts ({} removed)", before, filtered.len(), removed);
        }
        filtered
    }
}

/// Compile a list of pattern strings into a `GlobSet`, auto-enhancing bare
/// patterns (those without `/`) into `**/PATTERN` and `**/PATTERN/**`.
fn build_glob_set(patterns: &[String]) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        if pat.contains('/') {
            builder.add(Glob::new(pat)?);
        } else {
            builder.add(Glob::new(&format!("**/{pat}"))?);
            builder.add(Glob::new(&format!("**/{pat}/**"))?);
        }
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn filter(includes: &[&str], excludes: &[&str]) -> ArtifactFilter {
        ArtifactFilter::new(
            &includes.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &excludes.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
        .unwrap()
    }

    fn make_artifact(path: &str) -> Artifact {
        Artifact {
            path: PathBuf::from(path),
            build_system: "Test",
            artifact_dir: "target",
            size_bytes: 0,
        }
    }

    #[test]
    fn no_patterns_matches_everything() {
        let f = filter(&[], &[]);
        assert!(f.matches(Path::new("app/node_modules")));
        assert!(f.matches(Path::new("project/target")));
    }

    #[test]
    fn include_only() {
        let f = filter(&["node_modules"], &[]);
        assert!(f.matches(Path::new("app/node_modules")));
        assert!(!f.matches(Path::new("app/target")));
    }

    #[test]
    fn exclude_only() {
        let f = filter(&[], &["node_modules"]);
        assert!(!f.matches(Path::new("app/node_modules")));
        assert!(f.matches(Path::new("app/target")));
    }

    #[test]
    fn exclude_takes_precedence_over_include() {
        let f = filter(&["node_modules"], &["node_modules"]);
        assert!(!f.matches(Path::new("app/node_modules")));
    }

    #[test]
    fn bare_pattern_matches_ancestor_dir() {
        // wxyc* should match wxyc-app/target because **/wxyc*/** expands
        let f = filter(&[], &["wxyc*"]);
        assert!(!f.matches(Path::new("wxyc-app/target")));
        assert!(f.matches(Path::new("other-app/target")));
    }

    #[test]
    fn bare_pattern_matches_leaf() {
        // node_modules should match app/node_modules because **/node_modules expands
        let f = filter(&["node_modules"], &[]);
        assert!(f.matches(Path::new("app/node_modules")));
        assert!(f.matches(Path::new("deep/nested/app/node_modules")));
    }

    #[test]
    fn pattern_with_slash_used_as_is() {
        let f = filter(&["app/node_modules"], &[]);
        assert!(f.matches(Path::new("app/node_modules")));
        assert!(!f.matches(Path::new("other/node_modules")));
    }

    #[test]
    fn question_mark_glob() {
        let f = filter(&[], &["proj?"]);
        assert!(!f.matches(Path::new("projA/target")));
        assert!(f.matches(Path::new("project/target")));
    }

    #[test]
    fn double_star_glob() {
        let f = filter(&["**/deep/**/target"], &[]);
        assert!(f.matches(Path::new("a/deep/b/target")));
        assert!(!f.matches(Path::new("a/shallow/target")));
    }

    #[test]
    fn invalid_pattern_returns_error() {
        let result = ArtifactFilter::new(&[], &["[invalid".to_string()].to_vec());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid filter pattern"));
    }

    #[test]
    fn apply_strips_root_prefix() {
        let f = filter(&["node_modules"], &[]);
        let artifacts = vec![
            make_artifact("/home/user/projects/app/node_modules"),
            make_artifact("/home/user/projects/rust-app/target"),
        ];
        let root = Path::new("/home/user/projects");
        let filtered = f.apply(root, artifacts);
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].path,
            PathBuf::from("/home/user/projects/app/node_modules")
        );
    }

    #[test]
    fn apply_with_no_filters_keeps_all() {
        let f = filter(&[], &[]);
        let artifacts = vec![
            make_artifact("/root/a/target"),
            make_artifact("/root/b/node_modules"),
        ];
        let filtered = f.apply(Path::new("/root"), artifacts);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn multiple_include_patterns() {
        let f = filter(&["node_modules", "target"], &[]);
        assert!(f.matches(Path::new("app/node_modules")));
        assert!(f.matches(Path::new("app/target")));
        assert!(!f.matches(Path::new("app/.venv")));
    }

    #[test]
    fn multiple_exclude_patterns() {
        let f = filter(&[], &["wxyc*", "old-*"]);
        assert!(!f.matches(Path::new("wxyc-app/target")));
        assert!(!f.matches(Path::new("old-project/node_modules")));
        assert!(f.matches(Path::new("my-app/target")));
    }
}
