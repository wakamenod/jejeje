use crate::config::Config;
use crate::judge;
use anyhow::Result;
use std::{fs, path::Path};

/// `je download <url>` — サンプルケースのみを再取得して test/ に保存する。
pub async fn run(url: String) -> Result<()> {
    let config = Config::load()?;
    let client = crate::commands::new::build_client()?;

    let test_dir = Path::new(&config.test_directory);
    fs::create_dir_all(test_dir)?;

    println!("Downloading samples from {url}...");
    let samples = judge::fetch_samples(&url, &client).await?;

    if samples.is_empty() {
        println!("No samples found.");
        return Ok(());
    }

    for (i, sample) in samples.iter().enumerate() {
        let n = i + 1;
        let in_path = test_dir.join(format!("{n}.in"));
        let out_path = test_dir.join(format!("{n}.out"));
        fs::write(&in_path, &sample.input)?;
        fs::write(&out_path, &sample.output)?;
    }

    println!("Downloaded {} sample(s) to {}/", samples.len(), config.test_directory);
    Ok(())
}
