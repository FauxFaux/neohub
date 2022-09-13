use std::fs;
use std::io::Write;
use std::time::{Duration, SystemTime};

use anyhow::Result;

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
    let mut client = neohub::Client::new(
        "wss://192.168.178.37:4243",
        "7764994a-e10f-4f5e-bdb0-14b8861dcdc3",
    )?;
    loop {
        let live_data = client.get_live_data().await?;
        writer.write_all(format!("{} ", now()).as_bytes())?;
        serde_json::to_writer(&mut writer, &live_data)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        std::thread::sleep(Duration::from_secs(5));
    }
}
