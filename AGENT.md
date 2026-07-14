# AGENT.md

このファイルは AI コーディングエージェント向けの指示書です。

## プロジェクト概要

`je` — 競技プログラミング向け CLI ツール。サンプルケースの取得とテスト実行に特化。

- **言語**: Rust (edition 2024)
- **バイナリ名**: `je`（`src/main.rs` エントリポイント）
- **対応ジャッジ**: AtCoder / Codeforces / yukicoder / AOJ

## ディレクトリ構成

```
src/
├── cli.rs               # CLI 引数定義（clap derive）
├── main.rs              # エントリポイント
├── config.rs            # 設定ファイル (~/.config/jejeje/config.toml)
├── error.rs             # エラー型 (AppError)
├── meta.rs              # コンテストメタ (.je-meta.json) の読み書き
├── commands/            # サブコマンド実装
│   ├── prepare.rs       # je prepare
│   ├── test.rs          # je test
│   ├── contests.rs      # je contests
│   ├── info.rs          # je info
│   └── config.rs        # je config
└── judge/               # ジャッジごとのスクレイピング・API
    ├── mod.rs           # URL 解決・ディスパッチ
    ├── model.rs         # 共通データ型
    ├── atcoder.rs
    ├── codeforces.rs
    ├── aoj.rs
    └── yukicoder.rs
```

## 実装完了後に必ず実行すること

コードの変更が完了したら、**必ず最後に以下を実行**してグリーンであることを確認すること:

```bash
just ci
```

このコマンドは以下を順に実行する:

1. `cargo fmt --all -- --check` — フォーマットチェック
2. `cargo clippy --all-targets --all-features -- -D warnings` — リントチェック（警告をエラー扱い）
3. `cargo test` — ユニットテスト・統合テスト（ネットワーク不要分のみ）

`just ci` が失敗した場合は、エラーを修正してから再度 `just ci` を実行し、グリーンになるまで繰り返すこと。ユーザーに返答する前に `just ci` がパスしていることが必須条件。

### フォーマット自動修正

`cargo fmt` のエラーが出た場合は手動修正ではなく以下で自動修正する:

```bash
just fmt
```

### clippy の注意点

- `--all-targets` が付くため `tests/` 配下も検査対象になる
- ローカルで `cargo clippy` だけ実行しても CI と結果が一致しない場合があるため、必ず `just ci` 経由で実行すること

## コーディング規約

- エラーハンドリングは `anyhow::Result`（アプリ層）と `thiserror` による `AppError`（ライブラリ層）を使い分ける
- `#[ignore]` を付けたテストはネットワーク通信を伴う統合テスト。通常の `cargo test` では実行されない
- 新しいジャッジを追加する場合は `src/judge/` に専用モジュールを追加し、`mod.rs` のディスパッチに組み込む
