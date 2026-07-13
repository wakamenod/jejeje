use crate::commands::prepare::build_client;
use crate::judge::{fetch_contest_list, JudgeKind};
use owo_colors::OwoColorize;

pub async fn run(judge_str: String, limit: usize) -> anyhow::Result<()> {
    let client = build_client()?;
    let judge = match judge_str.as_str() {
        "atcoder" => JudgeKind::AtCoder,
        "codeforces" => JudgeKind::Codeforces,
        "yukicoder" => JudgeKind::Yukicoder,
        "aoj" => JudgeKind::Aoj,
        _ => return Err(anyhow::anyhow!("Unsupported judge: {}", judge_str)),
    };

    println!("Fetching contest list for {}...", judge.as_str().cyan());
    let mut contests = fetch_contest_list(&judge, &client).await?;
    contests.truncate(limit);

    if contests.is_empty() {
        println!("No contests found.");
        return Ok(());
    }

    for c in contests {
        println!("{} — {} ({})", c.id.green(), c.name, c.url.underline());
    }

    Ok(())
}
