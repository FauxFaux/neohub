use anyhow::Result;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let mut client = neohub::Client::from_env()?;
    let mut rl = rustyline::Editor::<()>::new()?;
    loop {
        let command = rl.readline(">> ")?;
        let result: Value = client.command_void(&command).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
}
