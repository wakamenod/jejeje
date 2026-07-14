use serde::{Deserialize, Serialize};

/// 問題ページから取得したサンプル入出力ペア。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleCase {
    pub input: String,
    pub output: String,
}

/// コンテスト一覧表示や曖昧検索で使用する、簡易的なコンテスト情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleContest {
    pub id: String,
    pub name: String,
    pub url: String,
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
    /// `je prepare` でコピーされたテンプレートファイル名（例: "main.cpp"）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub filename: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> TaskMeta {
        TaskMeta {
            id: "a".to_string(),
            name: "Two Sum".to_string(),
            url: "https://atcoder.jp/contests/abc001/tasks/abc001_a".to_string(),
            filename: None,
        }
    }

    fn sample_contest() -> ContestMeta {
        ContestMeta {
            judge: "atcoder".to_string(),
            contest_id: "abc001".to_string(),
            contest_name: "AtCoder Beginner Contest 001".to_string(),
            url: "https://atcoder.jp/contests/abc001".to_string(),
            tasks: vec![sample_task()],
        }
    }

    #[test]
    fn sample_case_json_roundtrip() {
        let sc = SampleCase {
            input: "3 5\n".to_string(),
            output: "8\n".to_string(),
        };
        let json = serde_json::to_string(&sc).unwrap();
        let sc2: SampleCase = serde_json::from_str(&json).unwrap();
        assert_eq!(sc.input, sc2.input);
        assert_eq!(sc.output, sc2.output);
    }

    #[test]
    fn task_meta_json_roundtrip() {
        let task = sample_task();
        let json = serde_json::to_string(&task).unwrap();
        let task2: TaskMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, task2.id);
        assert_eq!(task.name, task2.name);
        assert_eq!(task.url, task2.url);
    }

    #[test]
    fn contest_meta_json_roundtrip() {
        let contest = sample_contest();
        let json = serde_json::to_string_pretty(&contest).unwrap();
        let contest2: ContestMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(contest.judge, contest2.judge);
        assert_eq!(contest.contest_id, contest2.contest_id);
        assert_eq!(contest.contest_name, contest2.contest_name);
        assert_eq!(contest.url, contest2.url);
        assert_eq!(contest.tasks.len(), contest2.tasks.len());
        assert_eq!(contest.tasks[0].id, contest2.tasks[0].id);
    }

    #[test]
    fn simple_contest_json_roundtrip() {
        let sc = SimpleContest {
            id: "abc300".to_string(),
            name: "AtCoder Beginner Contest 300".to_string(),
            url: "https://atcoder.jp/contests/abc300".to_string(),
        };
        let json = serde_json::to_string(&sc).unwrap();
        let sc2: SimpleContest = serde_json::from_str(&json).unwrap();
        assert_eq!(sc.id, sc2.id);
        assert_eq!(sc.name, sc2.name);
        assert_eq!(sc.url, sc2.url);
    }

    #[test]
    fn simple_contest_deserialize_from_literal() {
        let json = r#"{"id":"cf1800","name":"Codeforces Round 1800","url":"https://codeforces.com/contest/1800"}"#;
        let sc: SimpleContest = serde_json::from_str(json).unwrap();
        assert_eq!(sc.id, "cf1800");
        assert_eq!(sc.name, "Codeforces Round 1800");
        assert_eq!(sc.url, "https://codeforces.com/contest/1800");
    }

    #[test]
    fn contest_meta_tasks_empty() {
        let contest = ContestMeta {
            judge: "codeforces".to_string(),
            contest_id: "1234".to_string(),
            contest_name: "Codeforces Round 1234".to_string(),
            url: "https://codeforces.com/contest/1234".to_string(),
            tasks: vec![],
        };
        let json = serde_json::to_string(&contest).unwrap();
        let c2: ContestMeta = serde_json::from_str(&json).unwrap();
        assert!(c2.tasks.is_empty());
    }
}
