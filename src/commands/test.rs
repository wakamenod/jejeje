use crate::meta;
use anyhow::Result;
use owo_colors::OwoColorize;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;

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
    /// プロセスの終了コード（シグナルで強制終了された場合は None）
    exit_code: Option<i32>,
    /// Unix シグナル番号（シグナルで強制終了された場合のみ Some）
    exit_signal: Option<i32>,
    /// 標準エラー出力の内容（空文字列の場合は None）
    stderr: Option<String>,
}

// ─── エントリポイント ──────────────────────────────────────────────

/// `je test` — テストケースを実行して AC / WA / TLE / RE を判定する。
pub async fn run(
    command: Option<String>,
    tle: f64,
    epsilon: Option<f64>,
    trim_trailing_whitespace: bool,
) -> Result<()> {
    let cmd = command.as_deref().unwrap_or("./a.out");
    let test_dir = Path::new("test");

    if !test_dir.exists() {
        anyhow::bail!(
            "Test directory '{}' not found. Run `je prepare` first.",
            test_dir.display()
        );
    }

    // メタ情報の表示（取得できない場合は静かにスキップ）
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(contest_meta) = meta::load(&cwd)
    {
        // CWD のディレクトリ名を task.id と照合して対応タスクを特定する
        let dir_name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(task) = contest_meta.tasks.iter().find(|t| t.id == dir_name) {
            println!("{}: {}", "Title".dimmed(), task.name.bold());
            println!("{}: {}", "URL  ".dimmed(), task.url);
            if let Some(fname) = detect_source_file(command.as_deref(), &cwd) {
                println!("{}: {}", "File ".dimmed(), fname.bold());
            }
            println!();
        }
    }

    // テストファイル収集（*.in を昇順ソート）
    let mut in_files: Vec<_> = std::fs::read_dir(test_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "in"))
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
                if compare(actual, &expected, epsilon, trim_trailing_whitespace) {
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
                print_diff(&input, &expected, outcome.actual.as_deref().unwrap_or(""));
            }
            Verdict::Tle => {
                println!("{label}: {} (>{:.0}ms)", verdict.display(), tle * 1000.0);
            }
            Verdict::Re => {
                println!("{label}: {} ({elapsed_ms}ms)", verdict.display());
                print_re_info(
                    outcome.exit_code,
                    outcome.exit_signal,
                    outcome.stderr.as_deref(),
                );
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
                actual: None,
                elapsed: start.elapsed(),
                exit_code: None,
                exit_signal: None,
                stderr: Some(format!("Failed to spawn process: {e}")),
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
                    exit_code: output.status.code(),
                    exit_signal: None,
                    stderr: None,
                })
            } else {
                // 終了コードの取得
                let exit_code = output.status.code();

                // Unix シグナル番号の取得（シグナルで強制終了された場合は code() が None）
                #[cfg(unix)]
                let exit_signal = output.status.signal();
                #[cfg(not(unix))]
                let exit_signal: Option<i32> = None;

                // stderr の取得（空なら None）
                let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
                let stderr = if stderr_str.trim().is_empty() {
                    None
                } else {
                    Some(stderr_str)
                };

                Ok(TestOutcome {
                    verdict: Verdict::Re,
                    actual: None,
                    elapsed,
                    exit_code,
                    exit_signal,
                    stderr,
                })
            }
        }
        Ok(Err(e)) => Ok(TestOutcome {
            verdict: Verdict::Re,
            actual: None,
            elapsed: start_for_tle.elapsed(),
            exit_code: None,
            exit_signal: None,
            stderr: Some(e.to_string()),
        }),
        Err(_) => {
            // TLE — timeout が発動した場合、Child はすでに drop されており kill 不要
            Ok(TestOutcome {
                verdict: Verdict::Tle,
                actual: None,
                elapsed: start_for_tle.elapsed(),
                exit_code: None,
                exit_signal: None,
                stderr: None,
            })
        }
    }
}

// ─── 出力比較 ──────────────────────────────────────────────────────

/// 実際の出力と期待出力を比較する。
/// `epsilon` が指定されている場合は浮動小数点の絶対誤差・相対誤差で比較する。
/// 改行コード (`\r\n` → `\n`) は常に正規化する。
/// `trim_trailing_whitespace` が true の場合、各行の末尾空白を除去してから比較する。
fn compare(
    actual: &str,
    expected: &str,
    epsilon: Option<f64>,
    trim_trailing_whitespace: bool,
) -> bool {
    // \r\n → \n の正規化（常時適用）
    let actual_owned;
    let expected_owned;
    let actual = if actual.contains('\r') {
        actual_owned = actual.replace("\r\n", "\n");
        actual_owned.as_str()
    } else {
        actual
    };
    let expected = if expected.contains('\r') {
        expected_owned = expected.replace("\r\n", "\n");
        expected_owned.as_str()
    } else {
        expected
    };

    // 行末空白の正規化（オプション）
    let actual_ls;
    let expected_ls;
    let actual = if trim_trailing_whitespace {
        actual_ls = actual
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        actual_ls.as_str()
    } else {
        actual
    };
    let expected = if trim_trailing_whitespace {
        expected_ls = expected
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        expected_ls.as_str()
    } else {
        expected
    };

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

// ─── ソースファイル検出 ────────────────────────────────────────────

/// 表示するソースファイル名を動的に決定する。
///
/// 優先順位:
/// 1. `command` の最後のトークンが既存ファイルを指す場合はそれを採用
///    (例: "ruby main.rb" → "main.rb")
/// 2. CWD に既知拡張子のソースファイルが 1 つだけあればそれを採用
fn detect_source_file(command: Option<&str>, cwd: &std::path::Path) -> Option<String> {
    const SOURCE_EXTENSIONS: &[&str] = &[
        "rb", "py", "cpp", "cc", "cxx", "c", "rs", "java", "go", "js", "ts", "kt", "swift", "cs",
        "hs", "ml", "scala", "d", "nim", "cr", "ex", "exs", "php", "pl",
    ];

    // 1. --command の最後のトークンがファイルとして存在するか確認
    if let Some(cmd) = command
        && let Some(last) = cmd.split_whitespace().last()
    {
        let candidate = cwd.join(last);
        if candidate.is_file() {
            return Some(last.to_string());
        }
    }

    // 2. CWD に既知拡張子のソースファイルが 1 つだけなら採用
    let source_files: Vec<String> = std::fs::read_dir(cwd)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|ext| SOURCE_EXTENSIONS.contains(&ext))
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    if source_files.len() == 1 {
        return Some(source_files.into_iter().next().unwrap());
    }

    None
}

// ─── RE 情報表示 ────────────────────────────────────────────────────

/// RE 判定時に終了コード・シグナル・stderr を表示する。
fn print_re_info(exit_code: Option<i32>, exit_signal: Option<i32>, stderr: Option<&str>) {
    print!("{}", format_re_info(exit_code, exit_signal, stderr));
}

/// RE 情報を文字列にフォーマットして返す（テスト容易性のために分離）。
fn format_re_info(
    exit_code: Option<i32>,
    exit_signal: Option<i32>,
    stderr: Option<&str>,
) -> String {
    let mut out = String::new();

    match (exit_code, exit_signal) {
        (_, Some(sig)) => {
            let name = signal_name(sig);
            out.push_str(&format!(
                "  {} {}\n",
                "signal:".dimmed(),
                format!("{sig} ({name})").bright_red()
            ));
        }
        (Some(code), None) => {
            out.push_str(&format!(
                "  {} {}\n",
                "exit code:".dimmed(),
                code.to_string().bright_red()
            ));
        }
        (None, None) => {}
    }

    if let Some(stderr) = stderr {
        const MAX_LINES: usize = 20;
        out.push_str(&format!("  {} :\n", "stderr".dimmed()));
        for line in stderr.lines().take(MAX_LINES) {
            out.push_str(&format!("    {}\n", line.bright_red()));
        }
        let total = stderr.lines().count();
        if total > MAX_LINES {
            out.push_str(&format!(
                "    {} ({} lines omitted)\n",
                "...".dimmed(),
                total - MAX_LINES
            ));
        }
    }

    out
}

/// Unix シグナル番号を人間が読める名前に変換する。
fn signal_name(sig: i32) -> &'static str {
    match sig {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        5 => "SIGTRAP",
        6 => "SIGABRT",
        7 => "SIGBUS",
        8 => "SIGFPE",
        9 => "SIGKILL",
        10 => "SIGUSR1",
        11 => "SIGSEGV",
        12 => "SIGUSR2",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        _ => "unknown",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── compare ─────────────────────────────────────────────────

    #[test]
    fn compare_identical() {
        assert!(compare("hello", "hello", None, false));
    }

    #[test]
    fn compare_trims_leading_trailing_whitespace() {
        assert!(compare("  hello\n", "hello", None, false));
    }

    #[test]
    fn compare_trims_both_sides() {
        assert!(compare("  42\n", "  42  ", None, false));
    }

    #[test]
    fn compare_different_strings() {
        assert!(!compare("hello", "world", None, false));
    }

    #[test]
    fn compare_case_sensitive() {
        assert!(!compare("Hello", "hello", None, false));
    }

    #[test]
    fn compare_newline_difference_after_trim() {
        // trim() は末尾改行も除くので、改行の有無は関係ない
        assert!(compare("42\n", "42", None, false));
    }

    #[test]
    fn compare_multiline_identical() {
        assert!(compare("1\n2\n3\n", "1\n2\n3", None, false));
    }

    #[test]
    fn compare_multiline_different() {
        assert!(!compare("1\n2\n3\n", "1\n2\n4", None, false));
    }

    // compare with epsilon ─────────────────────────────────────────

    #[test]
    fn compare_float_exact_match_no_epsilon_needed() {
        assert!(compare("3.14", "3.14", Some(1e-9), false));
    }

    #[test]
    fn compare_float_within_absolute_tolerance() {
        assert!(compare("1.0000001", "1.0", Some(1e-6), false));
    }

    #[test]
    fn compare_float_outside_tolerance() {
        assert!(!compare("1.01", "1.0", Some(1e-9), false));
    }

    #[test]
    fn compare_float_relative_tolerance() {
        // |1.0001 - 1.0| / |1.0| = 1e-4 <= 1e-3
        assert!(compare("1.0001", "1.0", Some(1e-3), false));
    }

    // CRLF normalization ───────────────────────────────────────────

    #[test]
    fn compare_crlf_actual_normalized() {
        // Windows 改行で出力されても LF に正規化して一致
        assert!(compare("42\r\n", "42\n", None, false));
    }

    #[test]
    fn compare_crlf_both_normalized() {
        assert!(compare("1\r\n2\r\n3\r\n", "1\n2\n3\n", None, false));
    }

    #[test]
    fn compare_crlf_expected_normalized() {
        // 期待値が CRLF でも正規化して比較できる
        assert!(compare("1\n2\n3\n", "1\r\n2\r\n3\r\n", None, false));
    }

    // trim_trailing_whitespace ─────────────────────────────────────

    #[test]
    fn compare_trim_trailing_whitespace_single_line() {
        // 行末スペースが実際の出力に含まれていても一致とみなす
        assert!(compare("hello   ", "hello", None, true));
    }

    #[test]
    fn compare_trim_trailing_whitespace_multiline() {
        assert!(compare("1  \n2   \n3\n", "1\n2\n3\n", None, true));
    }

    #[test]
    fn compare_no_trim_trailing_whitespace_fails() {
        // オプションなしでは行末スペースも比較対象（ただし全体 trim で末尾は消える）
        // 中間行の末尾スペースは消えないので不一致
        assert!(!compare("1  \n2   \n3", "1\n2\n3", None, false));
    }

    #[test]
    fn compare_trim_trailing_whitespace_crlf_combined() {
        // CRLF 正規化 + 行末空白除去の組み合わせ
        assert!(compare("1  \r\n2   \r\n3\r\n", "1\n2\n3\n", None, true));
    }

    // ─── compare_float ───────────────────────────────────────────

    #[test]
    fn compare_float_single_token_pass() {
        assert!(compare_float("3.14159", "3.14159", 1e-9));
    }

    #[test]
    fn compare_float_single_token_within_eps() {
        assert!(compare_float("1.000001", "1.0", 1e-5));
    }

    #[test]
    fn compare_float_single_token_outside_eps() {
        assert!(!compare_float("2.0", "1.0", 1e-9));
    }

    #[test]
    fn compare_float_multiple_tokens_all_pass() {
        assert!(compare_float("1.0 2.0 3.0", "1.0 2.0 3.0", 1e-9));
    }

    #[test]
    fn compare_float_multiple_tokens_one_fails() {
        assert!(!compare_float("1.0 2.0 99.0", "1.0 2.0 3.0", 1e-9));
    }

    #[test]
    fn compare_float_token_count_mismatch() {
        assert!(!compare_float("1.0 2.0", "1.0", 1e-9));
    }

    #[test]
    fn compare_float_non_numeric_exact_match() {
        // 数値でないトークンは文字列完全一致のみ
        assert!(compare_float("YES", "YES", 1e-9));
    }

    #[test]
    fn compare_float_non_numeric_mismatch() {
        assert!(!compare_float("YES", "NO", 1e-9));
    }

    #[test]
    fn compare_float_mixed_tokens() {
        assert!(compare_float("YES 3.14", "YES 3.14", 1e-9));
    }

    #[test]
    fn compare_float_near_zero_absolute_tolerance() {
        // ef が 0 に近い場合は絶対誤差で判定
        assert!(compare_float("0.0000001", "0.0", 1e-6));
    }

    // ─── signal_name ─────────────────────────────────────────────

    #[test]
    fn signal_name_known_signals() {
        assert_eq!(signal_name(1), "SIGHUP");
        assert_eq!(signal_name(2), "SIGINT");
        assert_eq!(signal_name(3), "SIGQUIT");
        assert_eq!(signal_name(4), "SIGILL");
        assert_eq!(signal_name(5), "SIGTRAP");
        assert_eq!(signal_name(6), "SIGABRT");
        assert_eq!(signal_name(7), "SIGBUS");
        assert_eq!(signal_name(8), "SIGFPE");
        assert_eq!(signal_name(9), "SIGKILL");
        assert_eq!(signal_name(10), "SIGUSR1");
        assert_eq!(signal_name(11), "SIGSEGV");
        assert_eq!(signal_name(12), "SIGUSR2");
        assert_eq!(signal_name(13), "SIGPIPE");
        assert_eq!(signal_name(14), "SIGALRM");
        assert_eq!(signal_name(15), "SIGTERM");
    }

    #[test]
    fn signal_name_unknown_falls_back() {
        assert_eq!(signal_name(0), "unknown");
        assert_eq!(signal_name(16), "unknown");
        assert_eq!(signal_name(99), "unknown");
        assert_eq!(signal_name(-1), "unknown");
    }

    // ─── format_re_info ──────────────────────────────────────────
    //
    // owo-colors が ANSI エスケープコードを埋め込むため、
    // プレーンテキスト部分が含まれているかを strip_ansi_codes で確認する。

    fn strip_ansi(s: &str) -> String {
        // ANSI エスケープシーケンス \x1b[...m を除去する簡易実装
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // '[' まで読み飛ばし、'm' が来るまで読み飛ばす
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == 'm' {
                            break;
                        }
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn format_re_info_exit_code_only() {
        let result = format_re_info(Some(1), None, None);
        let plain = strip_ansi(&result);
        assert!(
            plain.contains("exit code:"),
            "exit code: label should appear"
        );
        assert!(plain.contains('1'), "exit code value should appear");
        assert!(
            !plain.contains("signal:"),
            "signal: label should not appear"
        );
        assert!(
            !plain.contains("stderr"),
            "stderr section should not appear"
        );
    }

    #[test]
    fn format_re_info_exit_code_nonzero() {
        let result = format_re_info(Some(139), None, None);
        let plain = strip_ansi(&result);
        assert!(plain.contains("exit code:"));
        assert!(plain.contains("139"));
    }

    #[test]
    fn format_re_info_signal_only() {
        let result = format_re_info(None, Some(11), None);
        let plain = strip_ansi(&result);
        assert!(plain.contains("signal:"), "signal: label should appear");
        assert!(plain.contains("11"), "signal number should appear");
        assert!(plain.contains("SIGSEGV"), "signal name should appear");
        assert!(
            !plain.contains("exit code:"),
            "exit code: label should not appear"
        );
    }

    #[test]
    fn format_re_info_signal_takes_priority_over_exit_code() {
        // シグナルが Some の場合、exit_code の値に関わらずシグナル表示が優先される
        let result = format_re_info(Some(1), Some(9), None);
        let plain = strip_ansi(&result);
        assert!(plain.contains("signal:"), "signal: label should appear");
        assert!(plain.contains("SIGKILL"));
        assert!(
            !plain.contains("exit code:"),
            "exit code: label should not appear"
        );
    }

    #[test]
    fn format_re_info_both_none_produces_empty() {
        let result = format_re_info(None, None, None);
        assert!(result.is_empty(), "both None should produce empty string");
    }

    #[test]
    fn format_re_info_stderr_shown() {
        let result = format_re_info(Some(1), None, Some("error occurred\nsecond line"));
        let plain = strip_ansi(&result);
        assert!(
            plain.contains("stderr"),
            "stderr section header should appear"
        );
        assert!(
            plain.contains("error occurred"),
            "first stderr line should appear"
        );
        assert!(
            plain.contains("second line"),
            "second stderr line should appear"
        );
    }

    #[test]
    fn format_re_info_stderr_truncated_at_20_lines() {
        // 21 行の stderr を渡すと最後の 1 行が省略されることを確認
        let long_stderr = (1..=21)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = format_re_info(None, None, Some(&long_stderr));
        let plain = strip_ansi(&result);
        assert!(plain.contains("line 20"), "line 20 should be shown");
        assert!(!plain.contains("line 21"), "line 21 should be omitted");
        assert!(
            plain.contains("1 lines omitted"),
            "omitted count should appear"
        );
    }

    #[test]
    fn format_re_info_stderr_exactly_20_lines_no_omission() {
        let exactly_20 = (1..=20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = format_re_info(None, None, Some(&exactly_20));
        let plain = strip_ansi(&result);
        assert!(plain.contains("line 20"), "line 20 should be shown");
        assert!(
            !plain.contains("omitted"),
            "no omission message should appear"
        );
    }

    #[test]
    fn format_re_info_exit_code_and_stderr_combined() {
        let result = format_re_info(Some(2), None, Some("segmentation fault"));
        let plain = strip_ansi(&result);
        assert!(plain.contains("exit code:"));
        assert!(plain.contains('2'));
        assert!(plain.contains("stderr"));
        assert!(plain.contains("segmentation fault"));
    }

    // ─── execute ─────────────────────────────────────────────────

    #[tokio::test]
    async fn execute_nonexistent_command_returns_re_with_spawn_error() {
        let outcome = execute("__nonexistent_command_xyz__", "", Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(outcome.verdict, Verdict::Re);
        assert!(outcome.actual.is_none());
        let stderr = outcome.stderr.expect("stderr should contain spawn error");
        assert!(
            stderr.contains("Failed to spawn process"),
            "stderr was: {stderr}"
        );
        assert!(outcome.exit_code.is_none());
        assert!(outcome.exit_signal.is_none());
    }

    #[tokio::test]
    async fn execute_nonzero_exit_returns_re_with_exit_code() {
        // `false` コマンドは exit code 1 で終了する（どの Unix 環境にも存在する）
        let outcome = execute("false", "", Duration::from_secs(5)).await.unwrap();
        assert_eq!(outcome.verdict, Verdict::Re);
        assert_eq!(outcome.exit_code, Some(1));
        assert!(outcome.actual.is_none());
    }

    #[tokio::test]
    async fn execute_nonzero_exit_stderr_is_captured() {
        // stderr に書き込んで exit 1 するシェルスクリプトを一時ファイルで実行
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "#!/bin/sh").unwrap();
        writeln!(tmp, "echo 'error message' >&2").unwrap();
        writeln!(tmp, "exit 1").unwrap();
        // 実行権限を付与
        let path = tmp.path().to_str().unwrap().to_string();
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();

        let outcome = execute(&path, "", Duration::from_secs(5)).await.unwrap();
        assert_eq!(outcome.verdict, Verdict::Re);
        assert_eq!(outcome.exit_code, Some(1));
        let stderr = outcome.stderr.expect("stderr should be captured");
        assert!(stderr.contains("error message"), "stderr was: {stderr}");
    }

    #[tokio::test]
    async fn execute_success_returns_ac_with_stdout() {
        let outcome = execute("echo hello", "", Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(outcome.verdict, Verdict::Ac);
        assert_eq!(outcome.actual.as_deref(), Some("hello\n"));
        assert!(outcome.stderr.is_none());
        assert_eq!(outcome.exit_code, Some(0));
    }

    #[tokio::test]
    async fn execute_stdin_is_passed_to_process() {
        // cat はそのまま stdin を stdout に流す
        let outcome = execute("cat", "hello world\n", Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(outcome.verdict, Verdict::Ac);
        assert_eq!(outcome.actual.as_deref(), Some("hello world\n"));
    }

    #[tokio::test]
    async fn execute_tle_returns_tle_verdict() {
        let outcome = execute("sleep 60", "", Duration::from_millis(100))
            .await
            .unwrap();
        assert_eq!(outcome.verdict, Verdict::Tle);
        assert!(outcome.actual.is_none());
        assert!(outcome.stderr.is_none());
    }

    #[tokio::test]
    async fn execute_empty_stderr_is_none() {
        // `false` は stderr に何も出力せず exit 1 で終了する
        let outcome = execute("false", "", Duration::from_secs(5)).await.unwrap();
        assert_eq!(outcome.verdict, Verdict::Re);
        assert!(outcome.stderr.is_none(), "empty stderr should be None");
    }

    #[tokio::test]
    async fn execute_whitespace_only_stderr_is_none() {
        // 空白・改行のみの stderr も None 扱い — 一時スクリプトで確認
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "#!/bin/sh").unwrap();
        writeln!(tmp, "printf '  \\n' >&2").unwrap();
        writeln!(tmp, "exit 1").unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();

        let outcome = execute(&path, "", Duration::from_secs(5)).await.unwrap();
        assert_eq!(outcome.verdict, Verdict::Re);
        assert!(
            outcome.stderr.is_none(),
            "whitespace-only stderr should be None"
        );
    }
}
