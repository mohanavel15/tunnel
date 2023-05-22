use std::sync::Arc;
use std::collections::HashMap;

use actix::Addr;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::Mutex;

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

use models::{WsMessage, TunnelType};
use crate::tcp::{TcpConn, tcp_connect, Stop};

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
                                let message = WsMessage::deserialize(json_message);
                                match message {
                                    WsMessage::InvaildSession => {
                                        break
                                    },
                                    WsMessage::Ready(r) => {
                                        match r.protocal {
                                            TunnelType::TCP => println!("tcp://{}:{}", r.ip, r.port),
                                            TunnelType::UDP => println!("udp://{}:{}", r.ip, r.port),
                                        }
                                    },
                                    WsMessage::Connect(connection_id) => {
                                        println!("connecting: {}", connection_id);
                                        let mut connections = connections.lock().await;
                                        let addr = tcp_connect(port, connection_id.clone(), tx.clone()).await;
                                        connections.insert(connection_id, addr);
                                    },
                                    WsMessage::Disconnect(connection_id) => {
                                        println!("disconnecting: {}", connection_id);
                                        let mut connections = connections.lock().await;
                                        if let Some(addr) = connections.get(&connection_id) {
                                            addr.do_send(Stop{});
                                            connections.remove(&connection_id);
                                        }
                                    },
                                    WsMessage::Message(tunnel_message) => {
                                        let connections = connections.lock().await;
                                        if let Some(addr) = connections.get(&tunnel_message.connection_id) {
                                            addr.do_send(tunnel_message);
                                        }
                                    },
                                    WsMessage::Error(e) => {
                                        if e == "No available port" {
                                            println!("Server is full. try again later");
                                            break
                                        } else {
                                            println!("unknown error: {}", e);
                                            break
                                        }
                                    }
                                    _ => println!("got unexpected msg: {:?}", message),
                                }
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
