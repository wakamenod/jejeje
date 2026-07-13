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
    /// Set up directories and download sample cases from a contest or problem URL.
    #[command(long_about = "\
Set up directories and download sample cases from a contest or problem URL.

URL types:
  Contest URL  Creates one subdirectory per task and saves .je-meta.json
               e.g. https://atcoder.jp/contests/abc001
  Problem URL  Creates a single task directory inside the current contest root
               e.g. https://atcoder.jp/contests/abc001/tasks/abc001_a

File handling:
  Samples    test/*.in and test/*.out are always overwritten with the latest data
  Templates  Copied only when the destination file does not already exist
             (existing source files are never overwritten)")]
    Prepare {
        /// Contest URL or problem URL
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

    /// Show contest metadata for the current directory (reads .je-meta.json).
    Contest,

    /// List all tasks in the current contest (reads .je-meta.json).
    Tasks,

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
