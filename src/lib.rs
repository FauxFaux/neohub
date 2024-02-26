pub mod commands;
mod live_data;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, ensure, Context, Result};
use futures_util::{SinkExt, StreamExt};
use log::debug;
use rustls::client::danger;
use rustls::crypto::ring::default_provider;
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error, SignatureScheme};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::time::timeout;
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
    opts: Opts,
}

#[non_exhaustive]
pub struct Opts {
    pub timeout: Duration,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(15),
        }
    }
}

impl Client {
    pub fn from_env() -> Result<Self> {
        Self::new(env_var("NEOHUB_URL")?, env_var("NEOHUB_TOKEN")?)
    }

    pub fn new(url: impl ToString, token: impl ToString) -> Result<Self> {
        Self::new_opts(url, token, Opts::default())
    }

    pub fn new_opts(url: impl ToString, token: impl ToString, opts: Opts) -> Result<Self> {
        Ok(Client {
            url: url.to_string(),
            token: token.to_string(),
            conn: None,
            opts,
        })
    }

    #[inline]
    async fn ensure_connected(&mut self) -> Result<&mut WsStream> {
        if self.conn.is_none() {
            self.conn = Some(connect(&self.url).await?);
        }
        Ok(self.conn.as_mut().expect("we just set it"))
    }

    pub async fn raw_message(&mut self, msg: &str) -> Result<(String, String)> {
        timeout(self.opts.timeout, self.raw_message_inner(msg))
            .await
            .with_context(|| "timeout sending raw message")?
    }

    async fn raw_message_inner(&mut self, msg: &str) -> Result<(String, String)> {
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

        let conn = self.ensure_connected().await?;
        debug!("sending: {}", to_send);

        conn.feed(Message::Text(to_send)).await?;
        conn.flush().await?;

        debug!("receiving");
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
        serde_json::from_str(&resp).with_context(|| anyhow!("reading {:?}", resp))
    }

    pub async fn command_str<T: DeserializeOwned>(
        &mut self,
        command: &str,
        arg: &str,
    ) -> Result<T> {
        let (_, resp) = self
            .raw_message(&format!("{{'{command}':'{arg}'}}"))
            .await?;
        serde_json::from_str(&resp).with_context(|| anyhow!("reading {:?}", resp))
    }

    pub async fn identify(&mut self) -> Result<Identity> {
        let (device_id, resp) = self
            .raw_message(&serialise_void("FIRMWARE"))
            .await
            .with_context(|| "requesting FIRMWARE version")?;
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

        let shutdown_result = timeout(self.opts.timeout, conn.close(None))
            .await
            .with_context(|| "timeout disconnecting");

        self.conn = None;

        Ok(shutdown_result??)
    }
}

#[inline]
fn serialise_void(command: &str) -> String {
    format!("{{'{command}':0}}")
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

#[derive(Debug)]
struct IgnoreAllCertificateSecurity(WebPkiSupportedAlgorithms);

impl danger::ServerCertVerifier for IgnoreAllCertificateSecurity {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<danger::ServerCertVerified, Error> {
        Ok(danger::ServerCertVerified::assertion())
    }

    // copy-paste of WebPkiServerVerifier, the only other implementation of this trait
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<danger::HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, cert, dss, &self.0)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<danger::HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, cert, dss, &self.0)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_schemes()
    }
}

async fn connect(url: &str) -> Result<WsStream> {
    debug!("attempting connection");
    let connector = Connector::Rustls(Arc::new(
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(IgnoreAllCertificateSecurity(
                default_provider().signature_verification_algorithms,
            )))
            .with_no_client_auth(),
    ));
    let (conn, _) = connect_async_tls_with_config(url, None, true, Some(connector)).await?;
    debug!("connected");
    Ok(conn)
}

fn env_var(key: &'static str) -> Result<String> {
    std::env::var(key).with_context(|| anyhow!("env var required: {key:?}"))
}
