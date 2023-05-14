use serde::{Serialize, Deserialize};

pub enum TunnelType {
    TCP,
    UDP
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TunnelMessage {
    pub connection_id: String,
    pub data: Vec<u8>,
}

impl TunnelMessage {
    pub fn new(connection_id: String, data: Vec<u8>) -> Self {
        Self { connection_id, data }
    }

    pub fn serialize(&self) -> String {
        let json_message = serde_json::to_string(self).unwrap();
        json_message
    }

    pub fn deserialize(json_message: String) -> Self {
        let message: Self = serde_json::from_str(&json_message).unwrap();
        message
    }
}