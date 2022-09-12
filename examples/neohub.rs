use anyhow::Result;
use serde_json::Value;

fn main() -> Result<()> {
    let client = neohub::Client::new_host("192.168.178.37")?;
    // let bytes = client.raw_message(b"{\"INFO\":0}")?;
    // let bytes = String::from_utf8_lossy(&bytes);
    // let value: Value = serde_json::from_str(bytes.as_ref())?;
    // println!("{:#?}", value);
    println!("{:?}", client.get_profiles()?);
    Ok(())
}
