use anyhow::{Context, Result};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let mut client = neohub::Client::from_env()?;
    println!("Attempting to connect...");
    println!(
        "{:?}",
        client
            .identify()
            .await
            .with_context(|| "initial connection failed")?
    );
    let mut rl = rustyline::Editor::<()>::new()?;
    loop {
        let command = rl.readline(">> ")?;
        if command.is_empty() {
            break;
        };
        let mut parts = command.split(' ');
        let cmd = parts.next().expect("command isn't empty");
        let args: Vec<_> = parts.collect();

        let result: Value = client.command(cmd, &args).await?;

        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    client.disconnect().await?;

    Ok(())
}
