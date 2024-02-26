use std::fs;
use std::io::Write;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use serde_json::Value;

fn now() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("it's not the past")
        .as_millis()
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let unix_millis = now();
    let writer = fs::File::create(format!("logs.{unix_millis}.jsonl.zstd"))?;
    let mut writer = zstd::Encoder::new(writer, 9)?;
    let mut client = neohub::Client::from_env()?.connect().await?;
    loop {
        let live_data: Value = client.command_void(neohub::commands::GET_LIVE_DATA).await?;
        writer.write_all(format!("{} ", now()).as_bytes())?;
        serde_json::to_writer(&mut writer, &live_data)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        std::thread::sleep(Duration::from_secs(5));
    }
}
