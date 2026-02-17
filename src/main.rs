use std::collections::HashSet;
use std::io;
use std::process;

use clap::Parser;

use clean_builds::cli::Cli;
use clean_builds::delete::confirm_and_delete;
use clean_builds::output::{print_dry_run_footer, print_summary};
use clean_builds::scanner::scan;
use clean_builds::size::compute_sizes;

fn main() {
    let cli = Cli::parse();

    let root = match cli.path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot access '{}': {e}", cli.path.display());
            process::exit(1);
        }
    };

    let excludes: HashSet<String> = cli.exclude.into_iter().collect();

    let mut artifacts = scan(&root, &excludes);

    if artifacts.is_empty() {
        println!("No build artifacts found.");
        return;
    }

    compute_sizes(&mut artifacts);

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if let Err(e) = print_summary(&mut out, &artifacts, cli.verbose) {
        eprintln!("Error writing output: {e}");
        process::exit(1);
    }

    if cli.delete {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        match confirm_and_delete(&mut out, &mut input, &artifacts, cli.yes) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error during deletion: {e}");
                process::exit(1);
            }
        }
    } else {
        let _ = print_dry_run_footer(&mut out);
    }
}
