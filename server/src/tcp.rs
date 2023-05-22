use std::io;
use actix::{Actor, Context, Addr, StreamHandler, Message, Handler, AsyncContext, ActorContext, Running, io::{FramedWrite, WriteHandler}};
use models::{TunnelMessage, WsMessage};
use tokio::io::{split, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::FramedRead;
use uuid::Uuid;
use codec::TcpCodec;

use crate::ws::WsConn;

pub struct TcpConn {
    id: String,
    ws_addr: Addr<WsConn>,
    framed_write: FramedWrite<Vec<u8>, WriteHalf<TcpStream>, TcpCodec>,
}

impl TcpConn {
    fn new(id: String, ws_addr: Addr<WsConn>, framed_write: FramedWrite<Vec<u8>, WriteHalf<TcpStream>, TcpCodec>) -> Self {
        Self { id, ws_addr, framed_write }
    }
}

impl Actor for TcpConn{
    type Context = Context<TcpConn>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let result = self.ws_addr.try_send(TcpConnect { id: self.id.clone(), addr: ctx.address() });
        if let Err(_e) = result {
            ctx.stop();
        }
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.ws_addr.do_send(TcpDiconnect { id: self.id.clone() });
        Running::Stop
    }
}

impl StreamHandler<Result<Vec<u8>, io::Error>> for TcpConn {
    fn handle(&mut self, item: Result<Vec<u8>, io::Error>, _ctx: &mut Self::Context) {
        let buffer = item.unwrap();
        let tunnel_message = TunnelMessage::new(self.id.clone(), buffer);
        self.ws_addr.do_send(tunnel_message)
    }
}

impl WriteHandler<io::Error> for TcpConn {}

pub async fn start_tcp_server(port: u16, ws_addr: Addr<WsConn>) {
    let socket = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    while let Ok((stream, _)) = socket.accept().await {
        let id = Uuid::new_v4().to_string();
        let (read, write) = split(stream);
        TcpConn::create(|ctx| {
            let framed_read = FramedRead::new(read, TcpCodec{});
            TcpConn::add_stream(framed_read, ctx);
            let framed_write = FramedWrite::new(write, TcpCodec{}, ctx);
            TcpConn::new(id, ws_addr.clone(), framed_write)
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpConnect {
    pub id: String,
    pub addr: Addr<TcpConn>,
}

impl Handler<TcpConnect> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: TcpConnect, ctx: &mut Self::Context) {
        self.tcp_connections.insert(msg.id.clone(), msg.addr);
        ctx.text(WsMessage::Connect(msg.id).serialize());
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpDiconnect {
    pub id: String
}

impl Handler<TcpDiconnect> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: TcpDiconnect, ctx: &mut Self::Context) {
        self.tcp_connections.remove(&msg.id);
        ctx.text(WsMessage::Disconnect(msg.id).serialize());
    }
}

impl Handler<TunnelMessage> for TcpConn {
    type Result = ();

    fn handle(&mut self, msg: TunnelMessage, _ctx: &mut Self::Context) {
        self.framed_write.write(msg.data)
    }
}
