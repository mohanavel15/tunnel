use actix_web::web;
use actix_web_actors::ws;
use actix::{Actor, StreamHandler, AsyncContext, Handler};
use actix::prelude::Message;

use crate::AppState;

use uuid::Uuid;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use models::{TunnelMessage, TunnelType};

pub struct WsConn {
    pub tunnel_type: TunnelType,
    pub app_state: web::Data<AppState>,
    pub connections: Arc<Mutex<HashMap<String, Sender<TunnelMessage>>>>
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut ports = self.app_state.ports.lock().unwrap();
        if ports.len() == 0 {
            ctx.text(String::from("No available port"));
            ctx.close(None);
            return;
        }
        
        let port = ports.pop().unwrap();
        let addr = ctx.address();

        let connections = Arc::clone(&self.connections);

        match self.tunnel_type {
            TunnelType::TCP => {
                _ = thread::spawn(move || { start_tcp_tunnel(port, addr, connections) })
            }
            TunnelType::UDP => {}
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("disconnected");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                let tunnel_message = TunnelMessage::deserialize(text.into());
                let connections = self.connections.lock().unwrap();
                let client = connections.get(&tunnel_message.connection_id);
                if client.is_some() {
                    client.unwrap().send(tunnel_message).unwrap();
                }
            },
            _ => (),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

impl Handler<WsMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut ws::WebsocketContext<WsConn>) {
        println!("sending msg");
        ctx.text(msg.0);
    }
}

fn start_tcp_tunnel(port: u16, addr: actix::Addr<WsConn>, connection: Arc<Mutex<HashMap<String, Sender<TunnelMessage>>>>) {
    let listener = TcpListener::bind(("0.0.0.0", port));
    if listener.is_err() {
        return;
    }

    println!("listening on 0.0.0.0:{}", port);
    let listener = listener.unwrap();
    
    loop {
        let (mut stream, _) = listener.accept().unwrap();
        let (ctx, crx) = channel::<TunnelMessage>();
        let connection_id = Uuid::new_v4().to_string();
        connection.lock().unwrap().insert(connection_id.clone(), ctx);
        let addr = addr.clone();

        thread::spawn(move || {
            let connection_id = connection_id.clone();

            loop {
                match crx.try_recv() {
                    Ok(d) => { println!("sending to the client"); stream.write_all(&d.data).unwrap() },
                    Err(_) => {}
                }

                let mut buffer = [0; 1024];
                match stream.read(&mut buffer) {
                    Ok(n) => {
                        let tunnel_message = TunnelMessage::new(connection_id.clone(), buffer[0..n].to_vec());
                        if tunnel_message.data.len() != 0 {
                            let ws_msg = tunnel_message.serialize();
                            addr.do_send(WsMessage(ws_msg))
                        }
                    },
                    Err(_) => {
                        println!("connection with server was severed");
                        break;
                    }
                }
    
            thread::sleep(Duration::from_millis(50));
        }});

    }
}