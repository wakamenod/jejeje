# jejeje

English | [日本語](README.md)

**J**udge & **E**xecute **J**oint **E**xam **J**ustification **E**nvironment

A lightweight command-line tool for competitive programming, specializing in fetching sample cases and running tests against them. Ships as a single binary with no runtime dependencies.

## Features

- **Single binary** — No Python, Node.js, or other runtime required
- **Multi-judge support** — AtCoder / Codeforces / yukicoder / AOJ

## Installation

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
cp target/release/je ~/.local/bin/  # Copy to a directory in your PATH
```

## Usage

### Contest / Problem Setup

```bash
# Create directories, fetch samples, and save metadata from a contest URL
je prepare https://atcoder.jp/contests/abc001

# Generated directory structure:
# abc001/
# ├── .je-meta.json
# ├── a/
# │   └── test/
# │       ├── 1.in
# │       └── 1.out
# └── b/
#     └── test/
#         ├── 1.in
#         └── 1.out

# Pass a problem URL to add a single task (contest root is detected automatically)
je prepare https://atcoder.jp/contests/abc001/tasks/abc001_a

# You can also use a contest ID instead of a URL
je prepare abc300       # → https://atcoder.jp/contests/abc300
je prepare cf1800       # → https://codeforces.com/contest/1800
je prepare yuki400      # → https://yukicoder.me/contests/400
je prepare itp1         # → AOJ ITP1 course

# Fuzzy search by keyword (searches all judges in parallel)
je prepare "beginner 300"   # → resolves to AtCoder Beginner Contest 300
```

Supported ID patterns:

| Pattern | Example | Resolves to |
|---|---|---|
| `abc`, `arc`, `agc`, `ahc`, `apc`, `jsc`, `past` + number | `abc300` | `atcoder.jp/contests/abc300` |
| `cf` + number | `cf1800` | `codeforces.com/contest/1800` |
| `yuki` + number | `yuki400` | `yukicoder.me/contests/400` |
| AOJ course name (`itp1`, `alds1`, `dsl`, `grl`, `cgl`, `alpc`) | `itp1` | AOJ ITP1 course |
| Keyword | `"beginner 300"` | Fuzzy search across all judges |

> **Fuzzy search behavior**: If multiple contests match, a list of candidates is printed and the command exits with an error (non-interactive). Numeric-only input (e.g. `1800`) falls back to fuzzy search instead of direct resolution, to avoid ID conflicts between Codeforces and yukicoder.

> **Re-running prepare**
> Running `prepare` again on an existing task directory always refreshes the sample files (`test/*.in` / `test/*.out`).
> Template files are skipped if they already exist, so your solution code is never overwritten.

### Running Tests

```bash
# Test with the default command (./a.out)
je test

# Specify a custom command
je test -c "python3 main.py"
je test -c "g++ main.cpp -o a.out && ./a.out"

# Change the time limit (default: 2.0 seconds)
je test --tle 3.0

# Allow floating-point error tolerance
je test -e 1e-6
```

Example output:
```
1: AC (54ms)
2: WA (48ms)
  Input:
    3 5
  Expected:
    8
  Actual:
    9
3: TLE (>2000ms)

2 / 3 passed
```

### Listing Contests

```bash
# Show contest list for each judge (newest first)
je contests atcoder
je contests codeforces
je contests yukicoder
je contests aoj

# Limit the number of results (default: 20)
je contests atcoder --limit 5
```

### Viewing Contest Info

```bash
# Show contest info and task list (reads .je-meta.json)
je info
```

### Configuration

```bash
# Show all settings
je config

# Get a setting value
je config template_dir

# Set a setting value
je config template_dir ~/.config/jejeje/templates
```

#### Available Settings

| Key | Default | Description |
|---|---|---|
| `template_dir` | `~/.config/jejeje/templates` | Directory where template files are stored |

### Template Feature

Any files placed in `template_dir` are automatically copied into each task directory when `prepare` runs.
Existing files are skipped, so your solution code is never overwritten.

```
~/.config/jejeje/templates/
├── main.cpp   ┐
└── main.rs    ┘ All files are copied to each task directory (skipped if already exist)
```

```bash
je prepare https://atcoder.jp/contests/abc001
# → abc001/a/main.cpp, abc001/a/main.rs, abc001/b/main.cpp ... are placed automatically
```

## Development

```bash
$ just

Available recipes:
    build             # Debug build
    build-release     # Release build
    ci                # Run all CI checks (fmt → clippy → test)
    clean             # Remove build artifacts
    clippy            # Lint check
    default           # Show recipe list
    fmt               # Auto-format code
    fmt-check         # Format check (same as CI, no changes applied)
    install           # Install to ~/.cargo/bin/je
    release tag       # Create and push a tag → triggers the release workflow
    release-rerun tag # Manually re-run the release workflow for an existing tag
    test              # Run unit tests
    test-all          # Run all tests (unit + integration)
    test-integration  # Run integration tests (requires network)
```

## Supported Judges

| Judge | Sample fetch | Contest fetch | Contest list | Method |
|---|---|---|---|---|
| AtCoder | ✅ | ✅ | ✅ | HTML scraping / list: [AtCoder Problems API](https://kenkoooo.com/atcoder/) |
| Codeforces | ✅ | ✅ | ✅ | HTML scraping / list: official REST API |
| yukicoder | ✅ | ✅ | ✅ | Samples: HTML scraping / contest & list: official REST API |
| AOJ | ✅ | ✅ | ✅ | Official REST API |
