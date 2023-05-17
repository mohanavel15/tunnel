use actix_web::web;
use actix_web_actors::ws;
use actix::{Actor, spawn, StreamHandler, AsyncContext, Handler, Addr, Message};

use crate::AppState;
use crate::tcp::{TcpConn, start_tcp_server};

use std::collections::HashMap;
use models::{TunnelMessage, TunnelType};

pub struct WsConn {
    pub tunnel_type: TunnelType,
    pub app_state: web::Data<AppState>,
    pub tcp_connections: HashMap<String, Addr<TcpConn>>,
    // Todo pub udp_connections: HashMap<String, Addr<TcpConn>>,
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut ports = self.app_state.ports.lock().unwrap();
        if ports.is_empty() {
            ctx.text(String::from("No available port"));
            ctx.close(None);
            return;
        }
        
        let port = ports.pop().unwrap();
        let addr = ctx.address();

        match self.tunnel_type {
            TunnelType::TCP => _ = spawn(async move { 
                println!("starting future");
                start_tcp_server(port, addr).await;
            }),
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
                match self.tunnel_type {
                    TunnelType::TCP => {
                        if let Some(client_addr) = self.tcp_connections.get(&tunnel_message.connection_id) {
                            client_addr.do_send(WsMessage(tunnel_message.serialize()));
                        }
                    },
                    TunnelType::UDP => {}
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
