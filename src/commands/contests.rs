use crate::commands::prepare::build_client;
use crate::judge::{fetch_contest_list, JudgeKind};
use owo_colors::{OwoColorize, Stream};

pub async fn run(judge_str: String, limit: Option<usize>) -> anyhow::Result<()> {
    let client = build_client()?;
    let judge = match judge_str.as_str() {
        "atcoder" => JudgeKind::AtCoder,
        "codeforces" => JudgeKind::Codeforces,
        "yukicoder" => JudgeKind::Yukicoder,
        "aoj" => JudgeKind::Aoj,
        _ => return Err(anyhow::anyhow!("Unsupported judge: {}", judge_str)),
    };

    println!(
        "Fetching contest list for {}...",
        judge.as_str().if_supports_color(Stream::Stdout, |s| s.cyan())
    );
    let mut contests = fetch_contest_list(&judge, &client).await?;
    if let Some(n) = limit {
        contests.truncate(n);
    }

    if contests.is_empty() {
        println!("No contests found.");
        return Ok(());
    }

    for c in contests {
        println!(
            "{} — {} ({})",
            c.id.if_supports_color(Stream::Stdout, |s| s.green()),
            c.name,
            c.url.if_supports_color(Stream::Stdout, |s| s.underline()),
        );
    }

    Ok(())
}
