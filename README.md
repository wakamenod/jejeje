# je

競技プログラミング向けのコマンドラインツール。
サンプルケースの取得と取得したケースに対するテスト実行に特化した、シングルバイナリで動作する軽量ツールです。

## 特徴

- **シングルバイナリ** — Python や Node.js などの実行環境不要
- **マルチジャッジ対応** — AtCoder / Codeforces / yukicoder / AOJ をサポート

## インストール

```bash
cargo install --path .
```

または手動ビルド:

```bash
cargo build --release
cp target/release/je ~/.local/bin/  # PATH の通った場所へコピー
```

## 使い方

### コンテスト・問題のセットアップ

```bash
# コンテスト URL からディレクトリ作成・サンプル取得・メタデータ保存を一括実行
je prepare https://atcoder.jp/contests/abc001

# 生成されるディレクトリ構造:
# abc001/
# ├── .je-meta.json
# ├── a/
# │   └── test/
# │       ├── 1.in
# │       └── 1.out
# └── b/
#     └── test/
#         ├── 1.in
#         └── 1.out

# 問題 URL を渡すと単一タスクのディレクトリを追加（コンテストルートを自動検出）
je prepare https://atcoder.jp/contests/abc001/tasks/abc001_a

# URL の代わりにコンテスト ID を直接指定できる
je prepare abc300       # → https://atcoder.jp/contests/abc300
je prepare cf1800       # → https://codeforces.com/contest/1800
je prepare yuki400      # → https://yukicoder.me/contests/400
je prepare itp1         # → AOJ ITP1 コース

# キーワードで曖昧検索も可能（全ジャッジから並列検索）
je prepare "beginner 300"   # → AtCoder Beginner Contest 300 を解決
```

対応する ID パターン:

| パターン | 例 | 解決先 |
|---|---|---|
| `abc`, `arc`, `agc`, `ahc`, `apc`, `jsc`, `past` + 数字 | `abc300` | `atcoder.jp/contests/abc300` |
| `cf` + 数字 | `cf1800` | `codeforces.com/contest/1800` |
| `yuki` + 数字 | `yuki400` | `yukicoder.me/contests/400` |
| AOJ コース名 (`itp1`, `alds1`, `dsl`, `grl`, `cgl`, `alpc`) | `itp1` | AOJ ITP1 コース |
| キーワード | `"beginner 300"` | 全ジャッジから曖昧検索 |

> **曖昧検索の動作**: 複数のコンテストがマッチした場合は候補一覧を表示してエラー終了します（非インタラクティブ）。数字のみの入力（例: `1800`）は Codeforces と yukicoder の ID 衝突を避けるため、直接解決されず曖昧検索にフォールバックします。

> **再実行について**
> 既存のタスクディレクトリに対して再度 `prepare` を実行すると、
> サンプルファイル (`test/*.in` / `test/*.out`) は常に最新に更新されます。
> テンプレートファイルはすでに存在する場合はスキップされるため、
> 回答中のコードが上書きされることはありません。

### サンプルケースのテスト実行

```bash
# デフォルトコマンド (./a.out) でテスト
je test

# コマンドを指定
je test -c "python3 main.py"
je test -c "g++ main.cpp -o a.out && ./a.out"

# 時間制限を変更（デフォルト 2.0 秒）
je test --tle 3.0

# 浮動小数点の誤差を許容
je test -e 1e-6
```

出力例:
```
1: AC (54ms)
2: WA (48ms)
  Input:
    3 5
  Expected:
    8
  Actual:
    9
3: TLE (>2000ms)

2 / 3 passed
```

### コンテスト一覧の取得

```bash
# 各ジャッジのコンテスト一覧を表示（最新順）
je contests atcoder
je contests codeforces
je contests yukicoder
je contests aoj

# 表示件数を制限（デフォルト 20 件）
je contests atcoder --limit 5
```

### コンテスト情報の確認

```bash
# コンテスト情報とタスク一覧を表示（.je-meta.json を参照）
je info
```

### 設定

```bash
# 全設定を表示
je config

# 設定値を確認
je config template_dir

# 設定値を変更
je config template_dir ~/.config/jejeje/templates
```

#### 設定項目一覧

| キー | デフォルト値 | 説明 |
|---|---|---|
| `template_dir` | `~/.config/jejeje/templates` | テンプレートファイルの格納ディレクトリ |

### テンプレート機能

`template_dir` に直接ファイルを置くと、`prepare` 実行時にそのディレクトリ内のファイルが全てタスクディレクトリへコピーされます。
ファイルがすでに存在する場合はスキップされるため、回答中のコードが上書きされることはありません。

```
~/.config/jejeje/templates/
├── main.cpp   ┐
└── main.rs    ┘ タスクディレクトリへ全てコピーされる（既存の場合はスキップ）
```

```bash
je prepare https://atcoder.jp/contests/abc001
# → abc001/a/main.cpp, abc001/a/main.rs, abc001/b/main.cpp ... が自動配置される
```

## 開発・テスト

### ユニットテスト

```bash
cargo test
```

### 統合テスト（実サーバーへの HTTP リクエストあり）

通常の `cargo test` では実行されません。`--ignored` フラグを付けて明示的に実行します。

```bash
# prepare テストを全ジャッジで実行
cargo test --test integration_prepare -- --ignored

# contests テストを全ジャッジで実行
cargo test --test integration_contests -- --ignored

# 特定のジャッジのみ実行
cargo test --test integration_prepare atcoder -- --ignored
```

## CI / リリース

### CI (`.github/workflows/ci.yml`)

`main` ブランチへの push および pull request で自動実行されます。

```
fmt-check → clippy → test
```

ローカルで同じチェックを実行するには:

```bash
just ci
```

### リリース (`.github/workflows/release.yml`)

`v*` 形式のタグを push すると自動的に起動し、テスト通過後に 3 プラットフォームのバイナリを
GitHub Release に添付します。

| プラットフォーム | アーカイブ |
|---|---|
| Linux (x86_64) | `je-<tag>-linux-x86_64.tar.gz` |
| macOS (Universal Binary) | `je-<tag>-macos-universal.tar.gz` |
| Windows (x86_64) | `je-<tag>-windows-x86_64.zip` |

#### タグによるリリース（通常フロー）

```bash
git tag v0.1.0
git push origin v0.1.0
```

#### 手動再実行

ワークフローは **リポジトリへの write 権限を持つユーザー**のみ実行できます（オーナー・コラボレーター）。read 権限のみのユーザーは実行できません。

**GitHub UI から実行する場合:**

1. リポジトリの **Actions** タブを開く
2. 左サイドバーから **Release** をクリック
3. **Run workflow** ボタンをクリック
4. Branch: `main`、tag フィールドに対象タグ（例: `v0.1.0`）を入力
5. 緑の **Run workflow** ボタンで実行

**CLI から実行する場合:**

```bash
gh workflow run release.yml --ref main --field tag=v0.1.0
```

## 対応ジャッジ

| ジャッジ | サンプル取得 | コンテスト取得 | コンテスト一覧 | 取得方式 |
|---|---|---|---|---|
| AtCoder | ✅ | ✅ | ✅ | HTML スクレイピング / 一覧: [AtCoder Problems API](https://kenkoooo.com/atcoder/) |
| Codeforces | ✅ | ✅ | ✅ | HTML スクレイピング / 一覧: 公式 REST API |
| yukicoder | ✅ | ✅ | ✅ | サンプル: HTML スクレイピング / コンテスト・一覧: 公式 REST API |
| AOJ | ✅ | ✅ | ✅ | 公式 REST API |
