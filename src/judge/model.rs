use serde::{Deserialize, Serialize};

/// 問題ページから取得したサンプル入出力ペア。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleCase {
    pub input: String,
    pub output: String,
}

/// コンテスト内の個別タスク情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    /// タスク ID（例: "a", "b", "A", "B"）
    pub id: String,
    /// タスク名（例: "Two Sum"）
    pub name: String,
    /// 問題ページの URL
    pub url: String,
}

/// コンテスト全体のメタデータ。`.je-meta.json` として保存される。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestMeta {
    /// ジャッジ識別子（例: "atcoder"）
    pub judge: String,
    /// コンテスト ID（例: "abc001"）
    pub contest_id: String,
    /// コンテスト名（例: "AtCoder Beginner Contest 001"）
    pub contest_name: String,
    /// コンテストの URL
    pub url: String,
    /// タスク一覧
    pub tasks: Vec<TaskMeta>,
}
