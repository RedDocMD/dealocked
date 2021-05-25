use log::debug;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::error::Error;
use tempfile::tempfile;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::init(LevelFilter::Debug, Config::default())?;
    let file = tempfile().expect("Failed to create tempfile");
    let mut file = File::from_std(file);
    let thing = tokio::spawn(async move {
        const URL: &'static str = "http://localhost:4000";
        debug!("Getting {}", URL);
        let response = reqwest::get(URL).await.unwrap();
        let contents = response.bytes().await.unwrap();
        file.write_all(&contents)
            .await
            .expect("Failed to write to file");
    });
    thing.await?;
    Ok(())
}
