use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async_tls_with_config, Connector};

pub struct Client {
    host: SocketAddr,
}

impl Client {
    pub fn new_host(host: &str) -> Result<Self> {
        Ok(Self::new_addr((IpAddr::from_str(host)?, 4243)))
    }

    pub fn new_addr(host: impl Into<SocketAddr>) -> Self {
        Client { host: host.into() }
    }

    pub async fn raw_message(&self, msg: &str) -> Result<String> {
        let connector = Connector::NativeTls(
            native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()?,
        );
        let (conn, _) =
            connect_async_tls_with_config("wss://192.168.178.37:4243", None, Some(connector))
                .await?;
        let (mut write, mut read) = conn.split();
        // write.feed(Message::Close(None)).await?;
        // write.flush().await?;
        // 7764994a-e10f-4f5e-bdb0-14b8861dcdc3
        let middle = serde_json::to_string(&json!({
            "token": "7764994a-e10f-4f5e-bdb0-14b8861dcdc3",
            "COMMANDS": [
                { "COMMAND": msg, "COMMANDID": 1, }
            ]
        }))?;
        let outer = json!({
            "message_type": "hm_get_command_queue",
            "message": middle,
        });
        let line = serde_json::to_string(&outer)?;
        println!("{}", line);
        write.feed(Message::Text(line)).await?;
        write.flush().await?;
        // write.write_all(msg)?;
        // write.write_all(b"\0\r")?;
        // write.flush()?;
        // write.shutdown(Shutdown::Write)?;
        // let mut conn = io::BufReader::new(read);
        // let mut buf = Vec::with_capacity(4096);
        // conn.read_until(0, &mut buf)?;
        // while !buf.is_empty() && [b'\r', b'\n', b'\0'].contains(&buf[buf.len() - 1]) {
        //     buf.pop();
        // }
        let buf = read
            .next()
            .await
            .expect("some")
            .expect("success")
            .into_data();
        let resp: Value = serde_json::from_slice(&buf)?;
        let buf = resp
            .as_object()
            .expect("response object")
            .get("response")
            .expect("response present")
            .as_str()
            .expect("response string")
            .to_string();
        Ok(buf)
    }

    pub async fn command_void<T: DeserializeOwned>(&self, command: &str) -> Result<T> {
        let resp = self.raw_message(&format!("{{'{}':0}}", command)).await?;
        Ok(serde_json::from_str(&resp)?)
    }

    pub async fn get_profiles(&self) -> Result<HashMap<String, Profile>> {
        Ok(self.command_void("GET_PROFILES").await?)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Profile {
    // 1-..
    pub PROFILE_ID: u16,
    // 0
    pub P_TYPE: u16,
    pub info: ProfileInfo,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfileInfo {
    pub monday: ProfileInfoDay,
    pub tuesday: ProfileInfoDay,
    pub wednesday: ProfileInfoDay,
    pub thursday: ProfileInfoDay,
    pub friday: ProfileInfoDay,
    pub saturday: ProfileInfoDay,
    pub sunday: ProfileInfoDay,
}

type TempSpec = [Value; 4];

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfileInfoDay {
    wake: TempSpec,
    leave: TempSpec,
    #[serde(rename = "return")]
    ret: TempSpec,
    sleep: TempSpec,
}

// #[derive(Deserialize, Serialize, Debug)]
// pub struct ProfileInfoDay {
//     pub time: String,
//     pub set_temp: f32,
//     five: u16,
//     troo: bool,
// }
