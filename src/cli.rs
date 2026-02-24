use clap::Parser;
use std::path::PathBuf;

/// Recursively scan for and remove build artifacts.
///
/// By default, runs in dry-run mode showing a summary of artifacts found.
/// Use --delete to actually remove them.
#[derive(Parser, Debug)]
#[command(name = "clean-builds", version)]
pub struct Cli {
    /// Root directory to scan
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Actually delete artifacts (default is dry-run)
    #[arg(long)]
    pub delete: bool,

    /// Skip confirmation prompt (use with --delete)
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Show individual artifact paths
    #[arg(short, long)]
    pub verbose: bool,

    /// Include only artifacts matching glob pattern (repeatable)
    #[arg(long, value_name = "PATTERN")]
    pub include: Vec<String>,

    /// Exclude artifacts matching glob pattern (repeatable)
    #[arg(long, value_name = "PATTERN")]
    pub exclude: Vec<String>,

    /// Include only these build systems (repeatable, see --list-systems)
    #[arg(long, value_name = "ID", conflicts_with = "exclude_system")]
    pub system: Vec<String>,

    /// Exclude these build systems (repeatable, see --list-systems)
    #[arg(long, value_name = "ID", conflicts_with = "system")]
    pub exclude_system: Vec<String>,

    /// List available build system IDs and exit
    #[arg(long)]
    pub list_systems: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let cli = Cli::parse_from(["clean-builds"]);
        assert_eq!(cli.path, PathBuf::from("."));
        assert!(!cli.delete);
        assert!(!cli.yes);
        assert!(!cli.verbose);
        assert!(cli.include.is_empty());
        assert!(cli.exclude.is_empty());
        assert!(cli.system.is_empty());
        assert!(cli.exclude_system.is_empty());
        assert!(!cli.list_systems);
    }

    #[test]
    fn all_options() {
        let cli = Cli::parse_from([
            "clean-builds",
            "--delete",
            "-y",
            "-v",
            "--include",
            "node_modules",
            "--include",
            "target",
            "--exclude",
            "vendor*",
            "--exclude",
            "old-*",
            "/tmp/projects",
        ]);
        assert_eq!(cli.path, PathBuf::from("/tmp/projects"));
        assert!(cli.delete);
        assert!(cli.yes);
        assert!(cli.verbose);
        assert_eq!(cli.include, vec!["node_modules", "target"]);
        assert_eq!(cli.exclude, vec!["vendor*", "old-*"]);
    }

    #[test]
    fn include_flag() {
        let cli = Cli::parse_from([
            "clean-builds",
            "--include",
            "node_modules",
            "--include",
            "target",
        ]);
        assert_eq!(cli.include, vec!["node_modules", "target"]);
        assert!(cli.exclude.is_empty());
    }

    #[test]
    fn verbose_long_form() {
        let cli = Cli::parse_from(["clean-builds", "--verbose"]);
        assert!(cli.verbose);
    }

    #[test]
    fn system_flag() {
        let cli = Cli::parse_from([
            "clean-builds",
            "--system",
            "cargo",
            "--system",
            "node",
        ]);
        assert_eq!(cli.system, vec!["cargo", "node"]);
        assert!(cli.exclude_system.is_empty());
    }

    #[test]
    fn exclude_system_flag() {
        let cli = Cli::parse_from([
            "clean-builds",
            "--exclude-system",
            "python",
        ]);
        assert!(cli.system.is_empty());
        assert_eq!(cli.exclude_system, vec!["python"]);
    }

    #[test]
    fn list_systems_flag() {
        let cli = Cli::parse_from(["clean-builds", "--list-systems"]);
        assert!(cli.list_systems);
    }

    #[test]
    fn system_and_exclude_system_conflict() {
        let result = Cli::try_parse_from([
            "clean-builds",
            "--system",
            "cargo",
            "--exclude-system",
            "node",
        ]);
        assert!(result.is_err());
    }
}
