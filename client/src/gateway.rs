use std::sync::Arc;
use std::collections::HashMap;

use actix::Addr;
use tokio::sync::mpsc::{unbounded_channel};
use tokio::sync::Mutex;

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

use models::TunnelMessage;

use crate::tcp::{TcpConn, tcp_connect};

pub async fn start_tcp_tunnel(port: u16) {
    let (ws, _) = connect_async("ws://localhost:3000/tunnel/tcp").await.unwrap();
    let (mut write, mut read) = ws.split();
    
    let (tx, mut rx) = unbounded_channel();
    let connections = Arc::new(Mutex::new(HashMap::<String, Addr<TcpConn>>::new()));

    loop {
        tokio::select! {
            result = read.next() => {
                match result {
                    Some(Ok(message)) => {
                        match message {
                            Message::Text(json_message) => {
                                let message = TunnelMessage::deserialize(json_message);
                                let mut connections = connections.lock().await;
                                let tcp_addr = connections.get(&message.connection_id);
                                let stx = match tcp_addr {
                                    Some(addr) => addr.clone(),
                                    None => { 
                                        let addr = tcp_connect(port, message.connection_id.clone(), tx.clone()).await;
                                        connections.insert(message.connection_id.clone(), addr.clone());
                                        addr
                                    },
                                };

                                stx.do_send(message);
                            },
                            Message::Close(_) => {},
                            _ => {}
                        }
                    },
                    Some(Err(e)) => {
                        println!("Error: {}", e);
                        break;
                    },
                    None => break,
                }
            }
            message = rx.recv() => {
                match message {
                    Some(msg) => {
                        write.send(Message::Text(msg.serialize())).await.unwrap();
                    },
                    None => {}
                }
            }
        }
    }
}
