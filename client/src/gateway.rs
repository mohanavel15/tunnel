use std::sync::Arc;
use std::collections::HashMap;

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
use tokio::sync::Mutex;

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;

use models::TunnelMessage;

pub async fn start_tcp_tunnel(port: u16) {
    let (ws, _) = connect_async("ws://localhost:3000/tunnel/tcp").await.unwrap();
    let (mut write, mut read) = ws.split();
    let (tx, mut rx) = unbounded_channel();
    let connections = Arc::new(Mutex::new(HashMap::<String, UnboundedSender<TunnelMessage>>::new()));

    loop {
        tokio::select! {
            result = read.next() => {
                match result {
                    Some(Ok(message)) => {
                        match message {
                            Message::Text(json_message) => {
                                let message = TunnelMessage::deserialize(json_message);
                                let mut connections = connections.lock().await;
                                let sock_tx = connections.get(&message.connection_id);
                                let stx = match sock_tx {
                                    Some(stx) => stx,
                                    None => { 
                                        let (stx, srx) = unbounded_channel();
                                        connections.insert(message.connection_id.clone(), stx);
                                        let new = connect_tcp(port, message.connection_id.clone(), srx, tx.clone());
                                        tokio::spawn(new);
                                        connections.get(&message.connection_id).unwrap()
                                    },
                                };

                                stx.send(message).unwrap();
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

async fn connect_tcp(port: u16, connection_id: String, mut rx: UnboundedReceiver<TunnelMessage>, tx: UnboundedSender<TunnelMessage>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    
    let (read, write) = stream.split();
    loop {
        tokio::select! {
            data = rx.recv() => {
                match data {
                    Some(d) => { _ = write.try_write(&d.data) },
                    None => {}
                }
            },

            result = read.readable() => {
                match result {
                    Ok(()) => {
                        let mut buf = [0; 1024];
                        let n = read.try_read(&mut buf).unwrap();
                        let bytes = buf[0..n].to_vec();
                        if bytes.len() > 0 {
                            let tunnel_message = TunnelMessage::new(connection_id.clone(), bytes);
                            tx.send(tunnel_message).unwrap();
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }
}