use anyhow::{bail, Context, Result};
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
        let parts = command.split(' ').collect::<Vec<_>>();
        let result: Value = match parts.len() {
            1 => client.command_void(parts[0]).await?,
            2 => client.command_str(parts[0], parts[1]).await?,
            _ => bail!("unable to handle multiple arguments"),
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    client.disconnect().await?;

    Ok(())
}
