# je

競技プログラミング向けのコマンドラインツール。サンプルケースの取得とテスト実行に特化した、シングルバイナリで動作する軽量ツールです。

## 特徴

- **シングルバイナリ** — Python や Node.js などの実行環境不要
- **ログイン・提出機能なし** — CAPTCHA 対応が必要な機能を意図的に省き、常に安定動作
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
```

> **再実行について**
> 既存のタスクディレクトリに対して再度 `prepare` を実行すると、
> サンプルファイル (`test/*.in` / `test/*.out`) は常に最新に更新されます。
> テンプレートファイルはすでに存在する場合はスキップされるため、
> 回答中のコードが上書きされることはありません。

### テスト実行

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

### コンテスト情報の確認

```bash
# コンテスト情報を表示（.je-meta.json を参照）
je contest

# タスク一覧を表示
je tasks
```

### 設定

```bash
# 全設定を表示
je config

# 設定値を確認
je config test_directory

# 設定値を変更
je config test_directory tests
je config template_dir ~/.config/je/templates
```

#### 設定項目一覧

| キー | デフォルト値 | 説明 |
|---|---|---|
| `contest_directory` | `{contest_id}` | コンテストディレクトリ名 |
| `task_directory` | `{task_id}` | タスクディレクトリ名 |
| `test_directory` | `test` | サンプルケース格納ディレクトリ名 |
| `template_dir` | (なし) | テンプレートファイルの格納ディレクトリ |

### テンプレート機能

`template_dir` に直接ファイルを置くと、`prepare` 実行時にそのディレクトリ内のファイルが全てタスクディレクトリへコピーされます。
ファイルがすでに存在する場合はスキップされるため、回答中のコードが上書きされることはありません。

```
~/.config/je/templates/
├── main.cpp   ┐
└── main.rs    ┘ タスクディレクトリへ全てコピーされる（既存の場合はスキップ）
```

```bash
je config template_dir ~/.config/je/templates
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
# 全ジャッジのテストを実行
cargo test --test integration_prepare -- --ignored

# 特定のジャッジのみ実行
cargo test --test integration_prepare atcoder -- --ignored
cargo test --test integration_prepare codeforces -- --ignored
cargo test --test integration_prepare yukicoder -- --ignored
cargo test --test integration_prepare aoj -- --ignored
```

テスト対象:

| テスト | 種別 | URL |
|---|---|---|
| AtCoder コンテスト | `abc001` 全 4 問 | `atcoder.jp/contests/abc001` |
| AtCoder 旧 URL | 単問 | `abc001.contest.atcoder.jp/tasks/abc001_1` |
| Codeforces コンテスト | `contest/1` 全 3 問 | `codeforces.com/contest/1` |
| Codeforces 単問 | Problem A | `codeforces.com/contest/1/problem/A` |
| yukicoder コンテスト | `contests/1` | `yukicoder.me/contests/1` |
| yukicoder 単問 | No.1 | `yukicoder.me/problems/no/1` |
| AOJ コース | ITP1 全問 | `onlinejudge.u-aizu.ac.jp/courses/lesson/1/ITP1` |
| AOJ 旧 URL | 単問 | `judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A` |

## 対応ジャッジ

| ジャッジ | サンプル取得 | コンテスト取得 | 取得方式 |
|---|---|---|---|
| AtCoder | ✅ | ✅ | HTML スクレイピング |
| Codeforces | ✅ | ✅ | HTML スクレイピング |
| yukicoder | ✅ | ✅ | サンプル: HTML スクレイピング / コンテスト: 公式 REST API |
| AOJ | ✅ | ✅ | 公式 REST API |

---

## TODO

以下は今後の実装予定です。

### 🔴 優先度高（コア機能）

#### AtCoder スクレイパー実装 (`src/judge/atcoder.rs`)

- [x] `fetch_samples`: 問題ページの `<section>` ブロックから入力例・出力例をパース
  - [x] `<h3>` タグで "入力例" / "出力例" / "Sample Input" / "Sample Output" を検出
  - [x] `<pre>` タグからサンプルテキストを抽出（`<pre><code>` ネスト構造にも対応）
  - [x] 日本語・英語両対応
  - [x] 末尾改行の正規化（`\n` 1 つに統一）
  - [x] 入力例・出力例の件数不一致時にエラーを返す
- [x] `fetch_contest`: タスク一覧ページ (`/contests/{id}/tasks`) のテーブルをパース
  - [x] `#task-table tbody tr` からタスク ID・名前・URL を抽出
- [x] `fetch_contest`: コンテストトップページからコンテスト名を取得
- [x] 旧 URL 形式への対応 (`abc001.contest.atcoder.jp` 形式)
- [x] リクエスト間の待機処理（過負荷防止、1 秒程度）

#### Codeforces スクレイパー実装 (`src/judge/codeforces.rs`)

- [x] `fetch_samples`: 問題ページの `<div class="sample-test">` からサンプルをパース
  - [x] `div.input pre` と `div.output pre` を対応付け
  - [x] `<div class="title">` を含む実際の HTML 構造に対応
  - [x] 末尾改行の正規化（`\n` 1 つに統一）
- [x] `fetch_contest`: コンテストページの `table.problems` からタスク一覧をパース
- [x] Gym URL (`/gym/{id}`) の対応
- [x] Problemset URL (`/problemset/problem/{id}/{id}`) の対応（URL 判定）
- [x] リクエスト間の待機処理（過負荷防止、1 秒程度）

#### yukicoder 実装 (`src/judge/yukicoder.rs`)

- [x] `fetch_samples`: 問題ページの HTML をスクレイピングしてサンプルを取得
  - `/api/v1/problems/{no}/file/in` は `BearerAuth` 必須で認証なしでは利用不可
  - `div.sample > pre` からサンプル入出力を抽出（`pre[0]`=入力、`pre[1]`=出力）
- [x] `fetch_contest`: `/api/v1/contest/id/{id}` のレスポンスからタスク一覧を組み立て
  - `ProblemIdList` の各 ID に対して個別に問題情報を取得する処理
- [x] API エラーレスポンスのハンドリング
  - `AppError::ApiError { status, url, body }` バリアントを追加
  - `fetch_contest` (コンテスト・問題取得) と `fetch_samples` (HTML 取得) で
    HTTP 非成功ステータス時に `ApiError` を返す `api_get()` ヘルパーを導入

#### AOJ API 実装 (`src/judge/aoj.rs`)

- [x] `fetch_samples`: `judgedat.u-aizu.ac.jp/testcases/samples/{id}` で全サンプルを一括取得
  - エンドポイントホストを `judgeapi` → `judgedat.u-aizu.ac.jp` に修正
  - `/problems/{id}` でサンプル数を事前取得するロジックを廃止
  - レスポンスは配列形式 `[{"problemId": "...", "serial": N, "in": "...", "out": "..."}, ...]`
  - `ApiSample` のフィールド名を `input`/`output` → `in`/`out`（serde rename）に修正
- [x] `fetch_contest`: コース API のレスポンス形式を確認・修正
  - `GET /courses` でコース一覧を取得し `shortName` でフィルタ → 数値 `courseId` を取得
  - `GET /courses/{courseId}/topics` (HAL 形式) で `_embedded.topics[]` を取得
  - `GET /topics/{topicId}/problems` (HAL 形式) で `_embedded.problems[]` を取得
  - 問題 ID / トピック ID は `_links.self.href` の末尾セグメントから抽出
- [x] Volume URL への対応（`/volumes/{vol_no}` 形式）
  - `is_contest_url` が `/volumes/` を含む URL を認識するよう拡張
  - `extract_volume_id` ヘルパーで URL から Volume 番号を抽出
  - `GET /problems/volumes/{vol_no}?page=0&size={n}` で全問題を一括取得
  - `fetch_contest` が Volume URL を検出して `fetch_volume` に委譲

---

### 🟡 優先度中

#### `je prepare` の改善

- [ ] `contest_directory` / `task_directory` 設定値のプレースホルダー展開
  - 現状: 設定値 `{contest_id}` がそのままディレクトリ名になっている
  - 実装: `{contest_id}` → 実際のコンテスト ID に置換する処理を追加
  - 対応プレースホルダー: `{contest_id}`, `{task_id}`, `{judge}`
- [ ] コンテスト作成時の進捗表示の改善（タスクごとのダウンロード状況）
- [ ] すでにディレクトリが存在する場合の上書き確認プロンプト
- [ ] `--force` フラグによる強制上書きオプション

#### `je test` の改善

- [ ] 実行コマンドが存在しない場合の分かりやすいエラーメッセージ
- [ ] 特定のテストケースのみ実行するオプション（例: `je test 1 2`）
- [ ] テスト結果の詳細表示モード（`--verbose`）
- [ ] 改行コード正規化（Windows の `\r\n` を `\n` に統一して比較）
- [ ] 末尾空白の正規化オプション

#### エラーメッセージの改善

- [ ] HTTP エラー時のステータスコードと URL を含むメッセージ
- [ ] 対応外 URL に対してどのジャッジがサポートされているかを提示
- [ ] ネットワーク到達不能時の分かりやすいメッセージ

---

### 🟢 優先度低（将来対応）

#### 追加機能

- [ ] `je test` の並列実行オプション（`-j N`）
- [ ] インタラクティブ問題のテスト（`je test --reactive <judge_command>`）
- [ ] スペシャルジャッジ対応（`--judge-command` でカスタムチェッカーを指定）
- [ ] ストレステスト補助（`je stress <generator> <brute_command>`）

#### ユーザビリティ

- [ ] シェル補完スクリプトの生成（`je completions bash/zsh/fish`）
- [ ] `je config` での設定一覧のカラー表示
- [ ] `je tasks` でタスクの URL をクリッカブル表示（OSC 8 ハイパーリンク）
- [ ] コンテスト作成時に生成されたディレクトリへ `cd` するためのシェル関数の提供

#### テスト・CI

- [x] 各ジャッジスクレイパーのユニットテスト（HTML フィクスチャを使ったパーステスト）
- [x] 統合テスト（実際の問題 URL に対するエンドツーエンドテスト）
  - `tests/integration_prepare.rs` — `#[ignore]` で通常 CI から除外し、`--ignored` フラグで明示実行
  - AtCoder（コンテスト / 旧 URL）・Codeforces（コンテスト / 単問）・yukicoder（コンテスト / 単問）・AOJ（コース / 旧 URL）の全 8 ケース
- [ ] GitHub Actions による CI の設定
- [ ] `cargo clippy` / `cargo fmt` の CI チェック

#### ドキュメント

- [ ] `je --help` の出力をより詳細に（使用例の追加）
- [ ] 各ジャッジの対応 URL パターン一覧のドキュメント化
