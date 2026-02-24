# clean-builds

Recursively scan for and remove build artifacts across projects.

Scans a directory tree for build artifacts (like `target/`, `node_modules/`, `__pycache__/`) using build-system marker files to avoid false positives, then optionally deletes them.

## Installation

```sh
cargo install --path .
```

## Usage

```
clean-builds [OPTIONS] [PATH]

Arguments:
  [PATH]  Root directory to scan (default: current directory)

Options:
  --delete              Actually delete artifacts (default is dry-run)
  -y, --yes             Skip confirmation prompt (use with --delete)
  -v, --verbose         Show individual artifact paths
  --include <PATTERN>   Include only artifacts matching glob pattern (repeatable)
  --exclude <PATTERN>   Exclude artifacts matching glob pattern (repeatable)
  --system <ID>         Include only these build systems (repeatable, see --list-systems)
  --exclude-system <ID> Exclude these build systems (repeatable, see --list-systems)
  --list-systems        List available build system IDs and exit
  -h, --help            Help
```

### Dry-run (default)

```sh
clean-builds ~/Developer
```

Shows a summary table grouped by build system:

```
Build System    Count        Size
--------------  -----  ----------
Node.js            47      8.6 GB
Python            497    469.3 MB
Rust/Cargo          6      5.5 GB
--------------  -----  ----------
Total             550     14.5 GB

Run with --delete to remove these artifacts.
```

### Delete artifacts

```sh
clean-builds ~/Developer --delete
```

Shows the same summary, then prompts for confirmation before deleting.

### Delete without prompting (for scripting)

```sh
clean-builds ~/Developer --delete --yes
```

### Filtering with `--include` and `--exclude`

Only clean `node_modules` directories:

```sh
clean-builds ~/Developer --include 'node_modules'
```

Skip projects whose directory name starts with `wxyc`:

```sh
clean-builds ~/Developer --exclude 'wxyc*'
```

Combine both -- clean only `target` dirs, but skip a specific project:

```sh
clean-builds ~/Developer --include 'target' --exclude 'old-project*'
```

Patterns without `/` are automatically matched as path components anywhere in the
relative path. A bare pattern like `wxyc*` matches both leaf names
(`wxyc-app/node_modules` via the ancestor dir) and artifact names directly. Patterns
containing `/` are used as-is for explicit path control (e.g., `apps/*/target`).

Exclude takes precedence over include. If no `--include` is specified, all artifacts
are included. Both flags are repeatable.

### Filtering by build system

Filter by build system identity using `--system` and `--exclude-system`. These
flags are mutually exclusive. Use `--list-systems` to see available IDs.

Only clean Node.js artifacts:

```sh
clean-builds ~/Developer --system node
```

Clean everything except Python:

```sh
clean-builds ~/Developer --exclude-system python
```

Multiple systems can be specified:

```sh
clean-builds ~/Developer --system cargo --system node
```

Combine with glob filters for fine-grained control:

```sh
clean-builds ~/Developer --system node --exclude 'legacy-*'
```

List available system IDs:

```sh
clean-builds --list-systems
```

System IDs are matched case-insensitively.

### Verbose mode

```sh
clean-builds ~/Developer --verbose
```

Shows individual artifact paths and sizes under each build system group, plus
detailed diagnostic logging on stderr (artifact matches, filter decisions, per-artifact
sizes). Without `--verbose`, only pipeline stage progress is logged to stderr.

## Supported Build Systems

Each artifact directory is only matched when a marker file exists in its parent directory to prevent false positives.

| ID | Build System | Artifact Dirs | Marker Files |
|---|---|---|---|
| `bundler` | Ruby/Bundler | `vendor/bundle/` | `Gemfile` |
| `cabal` | Haskell/Cabal | `dist-newstyle/` | `*.cabal` |
| `cargo` | Rust/Cargo | `target/` | `Cargo.toml` |
| `cmake` | C/C++/CMake | `build/`, `CMakeFiles/` | `CMakeLists.txt` |
| `cocoapods` | CocoaPods | `Pods/` | `Podfile` |
| `composer` | PHP/Composer | `vendor/` | `composer.json` |
| `dotnet` | .NET/C# | `bin/`, `obj/` | `*.csproj` or `*.sln` |
| `flutter` | Dart/Flutter | `.dart_tool/`, `build/` | `pubspec.yaml` |
| `gradle` | Android/Gradle | `build/`, `.gradle/` | `build.gradle` or `build.gradle.kts` |
| `maven` | Java/Maven | `target/` | `pom.xml` |
| `mix` | Elixir/Mix | `_build/`, `deps/` | `mix.exs` |
| `node` | Node.js | `node_modules/`, `.next/`, `.nuxt/`, `.output/` | `package.json` |
| `python` | Python | `__pycache__/` (no marker), `.venv/`, `venv/`, `.mypy_cache/` (no marker), `.pytest_cache/` (no marker), `.tox/`, `*.egg-info/` | `pyproject.toml` or `setup.py` or `requirements.txt` (where noted) |
| `sbt` | Scala/SBT | `target/` | `build.sbt` |
| `spm` | Swift/SPM | `.build/` | `Package.swift` |
| `stack` | Haskell/Stack | `.stack-work/` | `stack.yaml` |
| `zig` | Zig | `zig-out/`, `zig-cache/` | `build.zig` |
