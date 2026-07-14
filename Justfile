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

# リントチェック（CI と同じ。警告をエラー扱い）
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# 通常テストを実行（#[ignore] のネットワークテストは除外）
test:
    cargo test

# ──────────────────────────────────────────────
# 開発用ユーティリティ
# ──────────────────────────────────────────────

# コードを自動フォーマット（チェックではなく適用）
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

# ネットワークを使う統合テストを実行（時間がかかる）
test-integration:
    cargo test -- --ignored

# 全テストを実行（通常 + 統合）
test-all:
    cargo test -- --include-ignored

# インストール（~/.cargo/bin/je に配置）
install:
    cargo install --path .
