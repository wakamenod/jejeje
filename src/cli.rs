use clap::{Parser, Subcommand};

/// Competitive programming helper — sample download & test runner
#[derive(Parser)]
#[command(name = "je", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a contest directory, download all samples, and save metadata.
    ///
    /// Accepts a contest URL (e.g. https://atcoder.jp/contests/abc001).
    /// Creates one subdirectory per task and populates test/ with sample cases.
    New {
        /// Contest URL
        url: String,

        /// Template name to copy into each task directory
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Add a single task directory with samples inside the current contest directory.
    ///
    /// Accepts a problem URL (e.g. https://atcoder.jp/contests/abc001/tasks/abc001_a).
    Add {
        /// Problem URL
        url: String,

        /// Template name to copy into the task directory
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Download (or re-download) sample cases for a problem URL into test/.
    Download {
        /// Problem URL
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
        /// Configuration key (e.g. test_directory)
        key: Option<String>,

        /// Value to set
        value: Option<String>,
    },
}
