use std::io;
use actix::{Actor, Context, Addr, StreamHandler, Message, Handler, AsyncContext, ActorContext, Running, io::{FramedWrite, WriteHandler}};
use actix_web::web::{BytesMut, BufMut};
use actix_codec::{Decoder, Encoder};
use models::TunnelMessage;
use tokio::io::{split, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::FramedRead;
use uuid::Uuid;

use crate::ws::{WsConn, WsMessage};

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
        println!("tcp started");
        let result = self.ws_addr.try_send(TcpConnect { id: self.id.clone(), addr: ctx.address() });
        if let Err(_e) = result {
            ctx.stop();
        }
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        println!("tcp stopped");
        self.ws_addr.do_send(TcpDiconnect { id: self.id.clone() });
        Running::Stop
    }
}

impl StreamHandler<Result<Vec<u8>, io::Error>> for TcpConn {
    fn handle(&mut self, item: Result<Vec<u8>, io::Error>, _ctx: &mut Self::Context) {
        println!("received buffer");
        let buffer = item.unwrap();
        let tunnel_message = TunnelMessage::new(self.id.clone(), buffer);
        self.ws_addr.do_send(WsMessage(tunnel_message.serialize()))
    }
}

impl WriteHandler<io::Error> for TcpConn {}

pub async fn start_tcp_server(port: u16, ws_addr: Addr<WsConn>) {
    println!("starting server");
    let socket = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("started server");
    while let Ok((stream, _)) = socket.accept().await {
        println!("new connection");
        let id = Uuid::new_v4().to_string();
        let (read, write) = split(stream);
        println!("stream split");
        TcpConn::create(|ctx| {
            println!("tcp create");
            let framed_read = FramedRead::new(read, TcpCodec{});
            TcpConn::add_stream(framed_read, ctx);
            let framed_write = FramedWrite::new(write, TcpCodec{}, ctx);
            TcpConn::new(id, ws_addr.clone(), framed_write)
        });
    }
}

struct TcpCodec {}

impl Encoder<Vec<u8>> for TcpCodec {
    type Error = io::Error;
    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let buffer = item.as_slice();
        dst.put_slice(buffer);
        Ok(())
    }
}

impl Decoder for TcpCodec {
    type Item = Vec<u8>;
    type Error = io::Error;
    
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        println!("tcp decode");
        let buffer = src.to_vec();
        if buffer.is_empty() {
            Ok(None)
        } else {
            Ok(Some(buffer))
        }
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

    fn handle(&mut self, msg: TcpConnect, _ctx: &mut Self::Context) {
        println!("adding new connections");
        self.tcp_connections.insert(msg.id, msg.addr);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpDiconnect {
    pub id: String
}

impl Handler<TcpDiconnect> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: TcpDiconnect, _ctx: &mut Self::Context) {
        println!("removing connections");
        self.tcp_connections.remove(&msg.id);
    }
}

impl Handler<WsMessage> for TcpConn {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, _ctx: &mut Self::Context) {
        println!("sending buffer");
        let tunnel_message = TunnelMessage::deserialize(msg.0);
        self.framed_write.write(tunnel_message.data);
    }
}