# jejeje Justfile
# 使い方: just <recipe>
# インストール: brew install just

# デフォルト: レシピ一覧を表示
default:
    @just --list

# ──────────────────────────────────────────────
# CI と同等のチェック（push 前に必ず実行）
# ──────────────────────────────────────────────

# CI と同じチェックを全て実行（fmt → clippy → test）
ci: fmt-check clippy test

# フォーマットチェック（CI と同じ。修正はしない）
fmt-check:
    cargo fmt --all -- --check

# リントチェック
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# 通常テストを実行
test:
    cargo test

# ──────────────────────────────────────────────
# 開発用ユーティリティ
# ──────────────────────────────────────────────

# コードを自動フォーマット
fmt:
    cargo fmt --all

# デバッグビルド
build:
    cargo build

# リリースビルド
build-release:
    cargo build --release

# ビルド成果物を削除
clean:
    cargo clean

# 統合テストを実行（実サーバーへの HTTP リクエストあり。時間がかかる）
# 通常の `cargo test` では実行されない #[ignore] テストのみ対象
#
# 全ジャッジの prepare / contests を網羅:
#   AtCoder    : abc001 全4問・旧URL形式
#   Codeforces : contest/1 全3問・単問
#   yukicoder  : contests/1・単問 No.1
#   AOJ        : ITP1 全問・旧URL形式
#   直接解決   : abc001 / cf1 / itp1
#   曖昧検索   : 複数マッチ・0件（エラー確認）
#   contests   : atcoder / codeforces / yukicoder / aoj
#
# 統合テストを実行
test-integration:
    cargo test --test integration_prepare -- --ignored
    cargo test --test integration_contests -- --ignored

# 全テストを実行（通常 + 統合）
test-all:
    cargo test -- --include-ignored

# インストール（~/.cargo/bin/je に配置）
install:
    cargo install --path .

# 使い方: just release v0.1.0
# タグを作成して push → GitHub Actions のリリースワークフローを起動
release tag:
    git tag {{tag}}
    git push origin {{tag}}

# リリースワークフローを手動で再実行（既存タグに対して）
# 使い方: just release-rerun v0.1.0
#
# GitHub UI から実行する場合:
#   1. リポジトリの Actions タブを開く
#   2. 左サイドバーから Release をクリック
#   3. Run workflow ボタンをクリック
#   4. Branch: main、tag フィールドに対象タグ（例: v0.1.0）を入力
#   5. 緑の Run workflow ボタンで実行
#
# リリースワークフローを手動で再実行（既存タグに対して）
release-rerun tag:
    gh workflow run release.yml --ref main --field tag={{tag}}
