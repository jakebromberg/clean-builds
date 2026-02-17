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
  --delete          Actually delete artifacts (default is dry-run)
  -y, --yes         Skip confirmation prompt (use with --delete)
  -v, --verbose     Show individual artifact paths
  --exclude <DIR>   Exclude directories from scanning (repeatable)
  -h, --help        Help
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

### Verbose mode

```sh
clean-builds ~/Developer --verbose
```

Shows individual artifact paths and sizes under each build system group.

## Supported Build Systems

Each artifact directory is only matched when a marker file exists in its parent directory to prevent false positives.

| Build System | Artifact Dirs | Marker Files |
|---|---|---|
| Java/Maven | `target/` | `pom.xml` |
| Rust/Cargo | `target/` | `Cargo.toml` |
| Scala/SBT | `target/` | `build.sbt` |
| Node.js | `node_modules/`, `.next/`, `.nuxt/`, `.output/` | `package.json` |
| Swift/SPM | `.build/` | `Package.swift` |
| Python | `__pycache__/` (no marker), `.venv/`, `venv/`, `.mypy_cache/` (no marker), `.pytest_cache/` (no marker), `.tox/`, `*.egg-info/` | `pyproject.toml` or `setup.py` or `requirements.txt` (where noted) |
| Android/Gradle | `build/`, `.gradle/` | `build.gradle` or `build.gradle.kts` |
| C/C++/CMake | `build/`, `CMakeFiles/` | `CMakeLists.txt` |
| .NET/C# | `bin/`, `obj/` | `*.csproj` or `*.sln` |
| Elixir/Mix | `_build/`, `deps/` | `mix.exs` |
| Haskell/Stack | `.stack-work/` | `stack.yaml` |
| Haskell/Cabal | `dist-newstyle/` | `*.cabal` |
| Dart/Flutter | `.dart_tool/`, `build/` | `pubspec.yaml` |
| Zig | `zig-out/`, `zig-cache/` | `build.zig` |
| PHP/Composer | `vendor/` | `composer.json` |
| CocoaPods | `Pods/` | `Podfile` |
| Ruby/Bundler | `vendor/bundle/` | `Gemfile` |
