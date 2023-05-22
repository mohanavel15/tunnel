use actix_web_actors::ws;
use actix::{Actor, spawn, StreamHandler, AsyncContext, Handler, Addr};
use tokio::task::JoinHandle;

use crate::tcp::{TcpConn, start_tcp_server};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use models::{TunnelMessage, TunnelType, WsMessage};

pub struct WsConn {
    pub tunnel_type: TunnelType,
    pub port: Option<u16>,
    pub server_task: Option<JoinHandle<()>>,
    pub ports: Arc<Mutex<Vec<u16>>>,
    pub tcp_connections: HashMap<String, Addr<TcpConn>>,
    // Todo pub udp_connections: HashMap<String, Addr<TcpConn>>,
}

impl WsConn {
    pub fn new(tunnel_type: TunnelType, ports: Arc<Mutex<Vec<u16>>>) -> Self {
        let port = None;
        let server_task = None;
        let tcp_connections = HashMap::new();
        Self {
            tunnel_type,
            port,
            ports,
            server_task,
            tcp_connections,
        }
    }
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut ports = self.ports.lock().unwrap();
        if ports.is_empty() {
            ctx.text(WsMessage::Error("No available port".into()).serialize());
            ctx.close(None);
            return;
        }
        
        let port = ports.pop().unwrap();
        let addr = ctx.address();

        self.port = Some(port);

        println!("using: {port}");

        match self.tunnel_type {
            TunnelType::TCP => self.server_task = Some(spawn(async move { 
                start_tcp_server(port, addr).await;
            })),
            TunnelType::UDP => {}
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(port) = self.port {
            if let Some(task) = &self.server_task {
                task.abort();
            }
            let mut ports = self.ports.lock().unwrap();
            ports.push(port);
            println!("port {port} freed");
        }
        println!("disconnected");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                let tunnel_message = TunnelMessage::deserialize(text.into());
                match self.tunnel_type {
                    TunnelType::TCP => {
                        if let Some(client_addr) = self.tcp_connections.get(&tunnel_message.connection_id) {
                            client_addr.do_send(tunnel_message);
                        }
                    },
                    TunnelType::UDP => {}
                }
            },
            _ => (),
        }
    }
}

impl Handler<TunnelMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: TunnelMessage, ctx: &mut ws::WebsocketContext<WsConn>) {
        ctx.text(WsMessage::Message(msg).serialize());
    }
}
