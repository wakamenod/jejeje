use crate::config::Config;
use anyhow::Result;
use owo_colors::OwoColorize;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

// ─── 判定結果 ───────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Ac,
    Wa,
    Tle,
    Re,
}

impl Verdict {
    fn display(&self) -> String {
        match self {
            Self::Ac => "AC".green().bold().to_string(),
            Self::Wa => "WA".red().bold().to_string(),
            Self::Tle => "TLE".yellow().bold().to_string(),
            Self::Re => "RE".bright_red().bold().to_string(),
        }
    }
}

// ─── テスト実行結果 ────────────────────────────────────────────────

struct TestOutcome {
    verdict: Verdict,
    actual: Option<String>,
    elapsed: Duration,
}

// ─── エントリポイント ──────────────────────────────────────────────

/// `je test` — テストケースを実行して AC / WA / TLE / RE を判定する。
pub async fn run(command: Option<String>, tle: f64, epsilon: Option<f64>) -> Result<()> {
    let config = Config::load()?;
    let cmd = command.as_deref().unwrap_or("./a.out");
    let test_dir = Path::new(&config.test_directory);

    if !test_dir.exists() {
        anyhow::bail!(
            "Test directory '{}' not found. Run `je download` first.",
            test_dir.display()
        );
    }

    // テストファイル収集（*.in を昇順ソート）
    let mut in_files: Vec<_> = std::fs::read_dir(test_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "in"))
        .collect();
    in_files.sort_by_key(|e| e.file_name());

    if in_files.is_empty() {
        println!("No test cases found in '{}'.", test_dir.display());
        return Ok(());
    }

    let total = in_files.len();
    let mut passed = 0usize;
    let time_limit = Duration::from_secs_f64(tle);

    for entry in &in_files {
        let in_path = entry.path();
        let out_path = in_path.with_extension("out");
        let label = in_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if !out_path.exists() {
            println!("{label}: {} (no .out file)", "SKIP".dimmed());
            continue;
        }

        let input = std::fs::read_to_string(&in_path)?;
        let expected = std::fs::read_to_string(&out_path)?;

        let outcome = execute(cmd, &input, time_limit).await?;

        // 出力比較（実行成功時のみ）
        let verdict = match &outcome.verdict {
            Verdict::Ac => {
                // execute() 内では出力があるだけで Ac になっていないため、ここで比較
                let actual = outcome.actual.as_deref().unwrap_or("");
                if compare(actual, &expected, epsilon) {
                    Verdict::Ac
                } else {
                    Verdict::Wa
                }
            }
            other => match other {
                Verdict::Tle => Verdict::Tle,
                Verdict::Re => Verdict::Re,
                _ => unreachable!(),
            },
        };

        let elapsed_ms = outcome.elapsed.as_millis();

        match &verdict {
            Verdict::Ac => {
                println!("{label}: {} ({elapsed_ms}ms)", verdict.display());
                passed += 1;
            }
            Verdict::Wa => {
                println!("{label}: {} ({elapsed_ms}ms)", verdict.display());
                print_diff(
                    &input,
                    &expected,
                    outcome.actual.as_deref().unwrap_or(""),
                );
            }
            Verdict::Tle => {
                println!(
                    "{label}: {} (>{:.0}ms)",
                    verdict.display(),
                    tle * 1000.0
                );
            }
            Verdict::Re => {
                println!("{label}: {} ({elapsed_ms}ms)", verdict.display());
            }
        }
    }

    // サマリー
    println!();
    if passed == total {
        println!("{}", format!("All {total} tests passed!").green().bold());
    } else {
        println!(
            "{} / {} passed",
            passed.to_string().bold(),
            total.to_string().bold()
        );
    }

    Ok(())
}

// ─── コマンド実行 ──────────────────────────────────────────────────

/// 指定コマンドに `input` を渡して実行し、結果を返す。
/// TLE の場合はプロセスを強制終了する。
async fn execute(cmd: &str, input: &str, time_limit: Duration) -> Result<TestOutcome> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let (program, args) = match parts.split_first() {
        Some(p) => p,
        None => anyhow::bail!("Empty command"),
    };

    let start = Instant::now();

    let mut child = match Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(TestOutcome {
                verdict: Verdict::Re,
                actual: Some(format!("Failed to spawn process: {e}")),
                elapsed: start.elapsed(),
            });
        }
    };

    // stdin に入力を書き込む
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes()).await;
    }

    // タイムアウト付きで待機
    // wait_with_output() は self を consume するため、TLE 時のために child_id を先に取得する
    let start_for_tle = start;
    match timeout(time_limit, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let elapsed = start_for_tle.elapsed();
            if output.status.success() {
                let actual = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(TestOutcome {
                    verdict: Verdict::Ac, // 比較は呼び出し元で行う
                    actual: Some(actual),
                    elapsed,
                })
            } else {
                Ok(TestOutcome {
                    verdict: Verdict::Re,
                    actual: None,
                    elapsed,
                })
            }
        }
        Ok(Err(e)) => Ok(TestOutcome {
            verdict: Verdict::Re,
            actual: Some(e.to_string()),
            elapsed: start_for_tle.elapsed(),
        }),
        Err(_) => {
            // TLE — timeout が発動した場合、Child はすでに drop されており kill 不要
            Ok(TestOutcome {
                verdict: Verdict::Tle,
                actual: None,
                elapsed: start_for_tle.elapsed(),
            })
        }
    }
}

// ─── 出力比較 ──────────────────────────────────────────────────────

/// 実際の出力と期待出力を比較する。
/// `epsilon` が指定されている場合は浮動小数点の絶対誤差・相対誤差で比較する。
fn compare(actual: &str, expected: &str, epsilon: Option<f64>) -> bool {
    let actual = actual.trim();
    let expected = expected.trim();

    if actual == expected {
        return true;
    }

    if let Some(eps) = epsilon {
        return compare_float(actual, expected, eps);
    }

    false
}

/// トークン単位で浮動小数点比較を行う。
fn compare_float(actual: &str, expected: &str, eps: f64) -> bool {
    let actual_tokens: Vec<&str> = actual.split_whitespace().collect();
    let expected_tokens: Vec<&str> = expected.split_whitespace().collect();

    if actual_tokens.len() != expected_tokens.len() {
        return false;
    }

    actual_tokens
        .iter()
        .zip(expected_tokens.iter())
        .all(|(a, e)| {
            if a == e {
                return true;
            }
            if let (Ok(af), Ok(ef)) = (a.parse::<f64>(), e.parse::<f64>()) {
                let diff = (af - ef).abs();
                // 絶対誤差または相対誤差が eps 以内
                diff <= eps || (ef.abs() > 1e-9 && diff / ef.abs() <= eps)
            } else {
                false
            }
        })
}

// ─── 差分表示 ──────────────────────────────────────────────────────

fn print_diff(input: &str, expected: &str, actual: &str) {
    const MAX_LINES: usize = 8;

    println!("  {} :", "Input".dimmed());
    for line in input.lines().take(MAX_LINES) {
        println!("    {line}");
    }

    println!("  {} :", "Expected".green());
    for line in expected.lines().take(MAX_LINES) {
        println!("    {}", line.green());
    }

    println!("  {} :", "Actual".red());
    for line in actual.lines().take(MAX_LINES) {
        println!("    {}", line.red());
    }
}
