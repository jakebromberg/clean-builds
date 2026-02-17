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

    /// Exclude directories from scanning (repeatable)
    #[arg(long, value_name = "DIR")]
    pub exclude: Vec<String>,
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
        assert!(cli.exclude.is_empty());
    }

    #[test]
    fn all_options() {
        let cli = Cli::parse_from([
            "clean-builds",
            "--delete",
            "-y",
            "-v",
            "--exclude",
            "vendor",
            "--exclude",
            ".git",
            "/tmp/projects",
        ]);
        assert_eq!(cli.path, PathBuf::from("/tmp/projects"));
        assert!(cli.delete);
        assert!(cli.yes);
        assert!(cli.verbose);
        assert_eq!(cli.exclude, vec!["vendor", ".git"]);
    }

    #[test]
    fn verbose_long_form() {
        let cli = Cli::parse_from(["clean-builds", "--verbose"]);
        assert!(cli.verbose);
    }
}
