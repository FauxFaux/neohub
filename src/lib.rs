pub mod commands;
mod live_data;

use anyhow::{anyhow, ensure, Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

pub use live_data::LiveData;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Client {
    url: String,
    token: String,
    conn: Option<WsStream>,
}

impl Client {
    pub fn from_env() -> Result<Self> {
        Self::new(std::env::var("NEOHUB_URL")?, std::env::var("NEOHUB_TOKEN")?)
    }

    pub fn new(url: impl ToString, token: impl ToString) -> Result<Self> {
        Ok(Client {
            url: url.to_string(),
            token: token.to_string(),
            conn: None,
        })
    }

    #[inline]
    async fn ensure_connected(&mut self) -> Result<()> {
        if self.conn.is_none() {
            self.conn = Some(connect(&self.url).await?);
        }
        Ok(())
    }

    pub async fn raw_message(&mut self, msg: &str) -> Result<(String, String)> {
        let middle = serde_json::to_string(&json!({
            "token": self.token,
            "COMMANDS": [
                { "COMMAND": msg, "COMMANDID": 1, }
            ]
        }))?;
        let outer = json!({
            "message_type": "hm_get_command_queue",
            "message": middle,
        });
        let to_send = serde_json::to_string(&outer)?;

        self.ensure_connected().await?;
        let conn = self
            .conn
            .as_mut()
            .expect("ensure_connected contract violated");

        conn.feed(Message::Text(to_send)).await?;
        conn.flush().await?;

        let buf = conn
            .next()
            .await
            .ok_or_else(|| anyhow!("no response received to command"))?
            .with_context(|| "unpacking websocket message")?
            .into_data();
        let resp: CommandResponse =
            serde_json::from_slice(&buf).with_context(|| "JSON-deserializing response")?;
        ensure!(
            resp.message_type == "hm_set_command_response" && resp.command_id == 1,
            "unexpected response type or id: {:?}",
            resp
        );
        Ok((resp.device_id, resp.response))
    }

    pub async fn command_void<T: DeserializeOwned>(&mut self, command: &str) -> Result<T> {
        let (_, resp) = self.raw_message(&serialise_void(command)).await?;
        Ok(serde_json::from_str(&resp).with_context(|| anyhow!("reading {:?}", resp))?)
    }

    pub async fn command_str<T: DeserializeOwned>(
        &mut self,
        command: &str,
        arg: &str,
    ) -> Result<T> {
        let (_, resp) = self
            .raw_message(&format!("{{'{}':'{}'}}", command, arg))
            .await?;
        Ok(serde_json::from_str(&resp).with_context(|| anyhow!("reading {:?}", resp))?)
    }

    pub async fn identify(&mut self) -> Result<Identity> {
        let (device_id, resp) = self.raw_message(&serialise_void("FIRMWARE")).await?;
        let firmware: Value = serde_json::from_str(&resp)?;
        Ok(Identity {
            device_id,
            firmware_version: firmware
                .get("firmware version")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
        })
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        let conn = match self.conn.as_mut() {
            None => return Ok(()),
            Some(conn) => conn,
        };

        let shutdown_result = conn.close(None).await;

        self.conn = None;

        Ok(shutdown_result?)
    }
}

#[inline]
fn serialise_void(command: &str) -> String {
    format!("{{'{}':0}}", command)
}

#[derive(Deserialize, Debug)]
struct CommandResponse {
    // we always send a fixed value (1)
    command_id: i64,

    // mac-address-like string
    device_id: String,

    // hm_set_command_response
    message_type: String,

    // json, in a string
    response: String,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub device_id: String,
    pub firmware_version: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Profile {
    // 1-..
    #[serde(rename = "PROFILE_ID")]
    pub profile_id: u16,
    // 0
    #[serde(rename = "P_TYPE")]
    pub p_type: u16,
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

async fn connect(url: &str) -> Result<WsStream> {
    let connector = Connector::NativeTls(
        native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()?,
    );
    let (conn, _) = connect_async_tls_with_config(url, None, Some(connector)).await?;
    Ok(conn)
}
