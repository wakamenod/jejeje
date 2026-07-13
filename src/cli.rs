use clap::{Parser, Subcommand};

/// Competitive programming helper — directory setup & test runner
#[derive(Parser)]
#[command(name = "je", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Set up directories and download sample cases from a contest/problem URL or query.
    #[command(long_about = "\
Set up directories and download sample cases from a contest/problem URL or query.

URL types & Queries:
  Contest URL  Creates one subdirectory per task and saves .je-meta.json
               e.g. https://atcoder.jp/contests/abc001
  Problem URL  Creates a single task directory inside the current contest root
               e.g. https://atcoder.jp/contests/abc001/tasks/abc001_a
  Query        Fuzzy lookup or direct resolution by ID (e.g. abc300, cf1800, itp1)

File handling:
  Samples    test/*.in and test/*.out are always overwritten with the latest data
  Templates  Copied only when the destination file does not already exist
             (existing source files are never overwritten)")]
    Prepare {
        /// Contest/Problem URL, or contest ID/query
        url: String,
    },

    /// Run sample test cases against your solution and report AC / WA / TLE / RE.
    Test {
        /// Command to execute (default: ./a.out)
        #[arg(short, long)]
        command: Option<String>,

        /// Time limit in seconds; exceeded runs are reported as TLE (default: 2.0)
        #[arg(long, default_value_t = 2.0)]
        tle: f64,

        /// Floating-point tolerance for answer comparison (e.g. 1e-6)
        #[arg(short, long)]
        epsilon: Option<f64>,
    },

    /// Show contest info and task list for the current directory (reads .je-meta.json).
    Info,

    /// List contests of a specific judge.
    Contests {
        /// Judge name (atcoder, codeforces, yukicoder, aoj)
        #[arg(value_parser = ["atcoder", "codeforces", "yukicoder", "aoj"])]
        judge: String,

        /// Limit the number of contests to show (default: unlimited)
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Show or set a configuration value.
    ///
    /// With no arguments, prints all current settings.
    /// With a key only, prints that setting.
    /// With key and value, updates the setting.
    Config {
        /// Configuration key (e.g. template_dir)
        key: Option<String>,

        /// Value to set
        value: Option<String>,
    },
}
