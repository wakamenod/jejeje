//! `je prepare` コマンドの統合テスト
//!
//! 実際のジャッジサーバーへ HTTP リクエストを送るため、通常の `cargo test` では
//! 実行されない。明示的に `--ignored` フラグを付けて実行すること。
//!
//! ```
//! # 全ケース実行
//! cargo test --test integration_prepare -- --ignored
//!
//! # 特定ジャッジのみ
//! cargo test --test integration_prepare atcoder -- --ignored
//! cargo test --test integration_prepare codeforces -- --ignored
//! cargo test --test integration_prepare yukicoder -- --ignored
//! cargo test --test integration_prepare aoj -- --ignored
//! ```
//!
//! 過剰なリクエストを避けるため、各ジャッジあたり 2 ケース（コンテスト or 旧URL込み）に絞っている。

use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

// ─── ヘルパー ───────────────────────────────────────────────────────────────

/// `je prepare <url>` を `dir` をカレントディレクトリとして実行する。
/// 成功時は TempDir を返す（ドロップするまでディレクトリが保持される）。
fn run_prepare(url: &str, dir: &TempDir) {
    Command::cargo_bin("je")
        .expect("je binary not found — run `cargo build` first")
        .args(["prepare", url])
        .current_dir(dir.path())
        .assert()
        .success();
}

/// `dir/task_id/test/` 以下にサンプルファイルが `min_count` 組以上存在することを確認する。
fn assert_samples_exist(base: &Path, task_id: &str, min_count: usize) {
    let test_dir = base.join(task_id).join("test");
    assert!(
        test_dir.exists(),
        "test/ directory not found: {}",
        test_dir.display()
    );
    for n in 1..=min_count {
        let in_file = test_dir.join(format!("{n}.in"));
        let out_file = test_dir.join(format!("{n}.out"));
        assert!(
            in_file.exists(),
            "sample input not found: {}",
            in_file.display()
        );
        assert!(
            out_file.exists(),
            "sample output not found: {}",
            out_file.display()
        );
    }
}

// ─── AtCoder ────────────────────────────────────────────────────────────────

/// ABC001 コンテスト URL から全4問のディレクトリ・サンプルが作成されること。
///
/// 検証:
/// - abc001/ ディレクトリと .je-meta.json が作成される
/// - abc001_1〜abc001_4 の各タスクディレクトリに test/1.in, test/1.out が存在する
#[test]
#[ignore]
fn atcoder_prepare_contest_abc001() {
    let dir = TempDir::new().unwrap();
    run_prepare("https://atcoder.jp/contests/abc001", &dir);

    let contest_dir = dir.path().join("abc001");
    assert!(contest_dir.exists(), "abc001/ not created");
    assert!(
        contest_dir.join(".je-meta.json").exists(),
        ".je-meta.json not found"
    );

    // ABC001 は 4 問。parse_task_table がアルファベット ID を小文字化するため
    // タスクディレクトリは a / b / c / d になる。
    for task_id in ["a", "b", "c", "d"] {
        assert_samples_exist(&contest_dir, task_id, 1);
    }
}

/// AtCoder 旧 URL（`{id}.contest.atcoder.jp`）での単一問題 prepare が動作すること。
///
/// 検証:
/// - 旧 URL が正規化されて fetch_samples が成功する
/// - abc001_1/test/1.in, abc001_1/test/1.out が作成される
#[test]
#[ignore]
fn atcoder_prepare_problem_legacy_url() {
    let dir = TempDir::new().unwrap();
    run_prepare(
        "https://abc001.contest.atcoder.jp/tasks/abc001_1",
        &dir,
    );

    assert_samples_exist(dir.path(), "abc001_1", 1);
}

// ─── Codeforces ─────────────────────────────────────────────────────────────

/// Codeforces Round 1 Problem A の単一問題 prepare が動作すること。
///
/// 検証:
/// - A/test/1.in, A/test/1.out が作成される
#[test]
#[ignore]
fn codeforces_prepare_problem() {
    let dir = TempDir::new().unwrap();
    run_prepare(
        "https://codeforces.com/contest/1/problem/A",
        &dir,
    );

    assert_samples_exist(dir.path(), "A", 1);
}

/// Codeforces コンテスト URL から全問のタスクディレクトリが作成されること。
///
/// 検証:
/// - 1/ 配下に .je-meta.json が作成される
/// - a / b / c の各タスクディレクトリにサンプルが存在する
///
/// Contest 1: Codeforces Beta Round 1 — 3 問（A/B/C）
///
/// Note: Codeforces gym は Cloudflare の Bot 検知によりヘッドレス HTTP クライアントから
/// アクセスできないため、通常のコンテスト URL でコンテスト取得機能を検証する。
#[test]
#[ignore]
fn codeforces_prepare_contest() {
    let dir = TempDir::new().unwrap();
    run_prepare("https://codeforces.com/contest/1", &dir);

    let contest_dir = dir.path().join("1");
    assert!(contest_dir.exists(), "1/ not created");
    assert!(
        contest_dir.join(".je-meta.json").exists(),
        ".je-meta.json not found"
    );

    // Contest 1 は A / B / C の 3 問
    for task_id in ["a", "b", "c"] {
        assert_samples_exist(&contest_dir, task_id, 1);
    }
}

// ─── yukicoder ──────────────────────────────────────────────────────────────

/// yukicoder コンテスト 1 から全問のサンプルが取得できること。
///
/// 検証:
/// - 1/ 配下にコンテストディレクトリ + .je-meta.json が作成される
/// - 各問題の test/ ディレクトリにサンプルが存在する
#[test]
#[ignore]
fn yukicoder_prepare_contest() {
    let dir = TempDir::new().unwrap();
    run_prepare("https://yukicoder.me/contests/1", &dir);

    let contest_dir = dir.path().join("1");
    assert!(contest_dir.exists(), "1/ not created");
    assert!(
        contest_dir.join(".je-meta.json").exists(),
        ".je-meta.json not found"
    );

    // 少なくとも1つのタスクが fetch できていること
    let task_dirs: Vec<_> = std::fs::read_dir(&contest_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    assert!(
        !task_dirs.is_empty(),
        "no task directories created under {}",
        contest_dir.display()
    );
}

/// yukicoder 問題 No.1 の単一問題 prepare が動作すること。
///
/// 検証:
/// - 1/test/1.in, 1/test/1.out が作成される
#[test]
#[ignore]
fn yukicoder_prepare_problem() {
    let dir = TempDir::new().unwrap();
    run_prepare("https://yukicoder.me/problems/no/1", &dir);

    assert_samples_exist(dir.path(), "1", 1);
}

// ─── AOJ ────────────────────────────────────────────────────────────────────

/// AOJ コース ITP1 の prepare が動作すること。
///
/// 検証:
/// - ITP1/ 配下に .je-meta.json が作成される
/// - ITP1_1_A など最低1つのタスクディレクトリが存在する
#[test]
#[ignore]
fn aoj_prepare_course_itp1() {
    let dir = TempDir::new().unwrap();
    run_prepare(
        "https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ITP1",
        &dir,
    );

    // コース ID が "ITP1" として contest_id に入るはず
    // （AOJ の extract_course_id によって決まる）
    let contest_dir = dir
        .path()
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .expect("No contest directory created");

    assert!(
        contest_dir.path().join(".je-meta.json").exists(),
        ".je-meta.json not found in {:?}",
        contest_dir.path()
    );

    // 少なくとも1つのタスクが fetch できていること
    let task_dirs: Vec<_> = std::fs::read_dir(contest_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    assert!(
        !task_dirs.is_empty(),
        "no task directories created under {:?}",
        contest_dir.path()
    );
}

/// AOJ 旧 URL（description.jsp?id=...）での単一問題 prepare が動作すること。
///
/// 検証:
/// - ITP1_1_A/test/1.in, ITP1_1_A/test/1.out が作成される
#[test]
#[ignore]
fn aoj_prepare_problem_legacy_url() {
    let dir = TempDir::new().unwrap();
    run_prepare(
        "https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A",
        &dir,
    );

    assert_samples_exist(dir.path(), "ITP1_1_A", 1);
}
