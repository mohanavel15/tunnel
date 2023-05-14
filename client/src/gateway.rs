use std::sync::Arc;
use std::collections::HashMap;

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
use tokio::sync::Mutex;

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use serde::{Serialize, Deserialize};
use serde_json::{json, from_slice};

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    connection_id: String,
    data: Vec<u8>,
}

pub async fn start_tcp_tunnel(port: u16) {
    let (ws, _) = connect_async("ws://localhost:3000/tcp").await.unwrap();
    let (mut write, mut read) = ws.split();
    let (tx, mut rx) = unbounded_channel();
    let connections = Arc::new(Mutex::new(HashMap::<String, UnboundedSender<Data>>::new()));

    loop {
        tokio::select! {
            result = read.next() => {
                match result {
                    Some(Ok(message)) => {
                        match message {
                            Message::Binary(bytes) => {
                                let message: Data = from_slice(&bytes).unwrap();
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
                        let bytes = json!(msg).to_string().as_bytes().to_vec();
                        write.send(Message::Binary(bytes)).await.unwrap();
                    },
                    None => {}
                }
            }
        }
    }
}

async fn connect_tcp(port: u16, connection_id: String, mut rx: UnboundedReceiver<Data>, tx: UnboundedSender<Data>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    
    let (read, write) = stream.split();

    let mut buf = [0; 1024];
    
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
                        let n = read.try_read(&mut buf).unwrap();
                        let bytes = buf[0..n].to_vec();
                        let data = Data{
                            connection_id: connection_id.clone(),
                            data: bytes,
                        };

                        tx.send(data).unwrap();
                    }
                    Err(_) => {}
                }
            }
        }
    }
}