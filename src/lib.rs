use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream};
use std::str::FromStr;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

pub struct Client {
    host: SocketAddr,
}

impl Client {
    pub fn new_host(host: &str) -> Result<Self> {
        Ok(Self::new_addr((IpAddr::from_str(host)?, 4242)))
    }

    pub fn new_addr(host: impl Into<SocketAddr>) -> Self {
        Client { host: host.into() }
    }

    pub fn raw_message(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let mut conn = TcpStream::connect(self.host)?;
        conn.write_all(msg)?;
        conn.write_all(b"\0\r")?;
        conn.flush()?;
        conn.shutdown(Shutdown::Write)?;
        let mut conn = io::BufReader::new(conn);
        let mut buf = Vec::with_capacity(4096);
        conn.read_until(0, &mut buf)?;
        while !buf.is_empty() && [b'\r', b'\n', b'\0'].contains(&buf[buf.len() - 1]) {
            buf.pop();
        }
        Ok(buf)
    }

    pub fn command_void<T: DeserializeOwned>(&self, command: &str) -> Result<T> {
        let resp = self.raw_message(format!("{{\"{}\":0}}", command).as_bytes())?;
        Ok(serde_json::from_slice(&resp)?)
    }

    pub fn get_profiles(&self) -> Result<HashMap<String, Profile>> {
        Ok(self.command_void("GET_PROFILES")?)
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
