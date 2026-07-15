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

## 開発

```bash
$ just

Available recipes:
    build             # デバッグビルド
    build-release     # リリースビルド
    ci                # CI と同じチェックを全て実行（fmt → clippy → test）
    clean             # ビルド成果物を削除
    clippy            # リントチェック
    default           # デフォルト: レシピ一覧を表示
    fmt               # コードを自動フォーマット
    fmt-check         # フォーマットチェック（CI と同じ。修正はしない）
    install           # インストール（~/.cargo/bin/je に配置）
    release tag       # タグを作成して push → GitHub Actions のリリースワークフローを起動
    release-rerun tag # リリースワークフローを手動で再実行（既存タグに対して）
    test              # 通常テストを実行
    test-all          # 全テストを実行（通常 + 統合）
    test-integration  # 統合テストを実行
```
