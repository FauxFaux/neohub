mod live_data;

use std::collections::HashMap;
use std::time::Duration;

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

    pub async fn raw_message(&mut self, msg: &str) -> Result<String> {
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
            resp.message_type == "hm_set_command_response",
            "unexpected response type: {:?}",
            resp
        );
        Ok(resp.response)
    }

    pub async fn command_void<T: DeserializeOwned>(&mut self, command: &str) -> Result<T> {
        let resp = self.raw_message(&format!("{{'{}':0}}", command)).await?;
        Ok(serde_json::from_str(&resp).with_context(|| anyhow!("reading {:?}", resp))?)
    }

    pub async fn get_profiles(&mut self) -> Result<HashMap<String, Profile>> {
        Ok(self.command_void("GET_PROFILES").await?)
    }

    pub async fn get_live_data(&mut self) -> Result<LiveData> {
        Ok(self.command_void("GET_LIVE_DATA").await?)
    }
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

async fn connect(url: &str) -> Result<WsStream> {
    let connector = Connector::NativeTls(
        native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()?,
    );
    let (mut conn, _) = connect_async_tls_with_config(url, None, Some(connector)).await?;
    Ok(conn)
}
