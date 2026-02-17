# clean-builds

Recursive build artifact cleaner. Scans directories for build artifacts using
build-system marker files to avoid false positives, then optionally deletes them.

## Architecture

- `cli.rs` - clap derive CLI definitions
- `rules.rs` - Declarative artifact rule registry
- `scanner.rs` - Recursive traversal and artifact detection
- `size.rs` - Parallel directory size computation
- `output.rs` - Human-readable output formatting
- `delete.rs` - Deletion logic with confirmation prompt

## Conventions

- Library-first design: logic in lib, thin main.rs
- Output functions take `&mut dyn Write` for testability
- `thiserror` for error types
- `clap` derive for CLI
- Integration tests use `tempfile` + `assert_cmd`
