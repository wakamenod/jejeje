//! `je contests` コマンドの統合テスト
//!
//! ネットワーク不要のテスト（引数バリデーション、ヘルプ表示）と
//! ネットワーク必要なテスト（`#[ignore]` 付き）を分離している。
//!
//! ```
//! # ネットワーク不要テスト
//! cargo test --test integration_contests
//!
//! # ネットワーク必要テストを含む全テスト
//! cargo test --test integration_contests -- --ignored
//! ```

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;

// ─── ネットワーク不要テスト ──────────────────────────────────────

/// judge 引数なしで実行するとエラーになること。
#[test]
fn contests_missing_judge_fails() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .arg("contests")
        .assert()
        .failure()
        .stderr(predicates::str::contains("required"));
}

/// 不正なジャッジ名を指定するとエラーになること。
#[test]
fn contests_invalid_judge_fails() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .args(["contests", "leetcode"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid value"));
}

/// --limit に数字以外を渡すとエラーになること。
#[test]
fn contests_invalid_limit_fails() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .args(["contests", "atcoder", "--limit", "abc"])
        .assert()
        .failure();
}

/// 有効なジャッジ名が value_parser で受理されること（atcoder / codeforces / yukicoder / aoj）。
/// ここではヘルプ出力に各ジャッジ名が含まれていることで間接的に確認する。
#[test]
fn contests_help_shows_judge_options() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .args(["contests", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("atcoder"))
        .stdout(predicates::str::contains("codeforces"))
        .stdout(predicates::str::contains("yukicoder"))
        .stdout(predicates::str::contains("aoj"));
}

// ─── NO_COLOR テスト ─────────────────────────────────────────────

/// NO_COLOR=1 を設定しても引数バリデーションが通常どおり動作すること。
#[test]
fn contests_no_color_env_does_not_affect_validation() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .env("NO_COLOR", "1")
        .args(["contests", "leetcode"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid value"));
}

/// NO_COLOR=1 を設定すると stdout に ANSI エスケープシーケンスが含まれないこと。
#[test]
#[ignore]
fn contests_no_color_env_suppresses_ansi() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .env("NO_COLOR", "1")
        .args(["contests", "atcoder", "--limit", "3"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b").not());
}

// ─── ネットワーク必要テスト ──────────────────────────────────────

/// `je contests atcoder --limit 3` が正常に実行でき、出力が得られること。
#[test]
#[ignore]
fn contests_atcoder_fetches_list() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .args(["contests", "atcoder", "--limit", "3"])
        .assert()
        .success()
        .stdout(predicates::str::contains("atcoder.jp"));
}

/// `je contests codeforces --limit 3` が正常に実行できること。
#[test]
#[ignore]
fn contests_codeforces_fetches_list() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .args(["contests", "codeforces", "--limit", "3"])
        .assert()
        .success()
        .stdout(predicates::str::contains("codeforces.com"));
}

/// `je contests yukicoder --limit 3` が正常に実行できること。
#[test]
#[ignore]
fn contests_yukicoder_fetches_list() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .args(["contests", "yukicoder", "--limit", "3"])
        .assert()
        .success()
        .stdout(predicates::str::contains("yukicoder.me"));
}

/// `je contests aoj --limit 3` が正常に実行できること。
#[test]
#[ignore]
fn contests_aoj_fetches_list() {
    Command::cargo_bin("je")
        .expect("je binary not found")
        .args(["contests", "aoj", "--limit", "3"])
        .assert()
        .success()
        .stdout(predicates::str::contains("u-aizu.ac.jp"));
}
