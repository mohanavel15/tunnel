use serde::{Serialize, Deserialize};
use actix::Message;

#[derive(Serialize, Deserialize, Debug)]
pub enum TunnelType {
    TCP,
    UDP
}

#[derive(Serialize, Deserialize, Debug)]
#[derive(Message)]
#[rtype(result = "()")]
pub struct TunnelMessage {
    pub connection_id: String,
    pub data: Vec<u8>,
}

impl TunnelMessage {
    pub fn new(connection_id: String, data: Vec<u8>) -> Self {
        Self { connection_id, data }
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn deserialize(json_message: String) -> Self {
        serde_json::from_str(&json_message).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WsJoin { 
    pub token: String, 
    pub protocal: TunnelType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WsReady {
    pub protocal: TunnelType,
    pub ip: String,
    pub port: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WsMessage {
    Join(WsJoin),
    InvaildSession,
    Ready(WsReady),
    Connect(String),
    Disconnect(String),
    Message(TunnelMessage),
    Error(String)
}

impl WsMessage {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn deserialize(json_message: String) -> Self {
        serde_json::from_str(&json_message).unwrap()
    }
}