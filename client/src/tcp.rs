use std::io;
use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};
use actix::{Actor, Context, StreamHandler, io::{FramedWrite, WriteHandler}, Addr, Handler, ActorContext, Message};
use models::TunnelMessage;
use tokio::io::{split, WriteHalf};
use tokio_util::codec::FramedRead;
use codec::TcpCodec;

pub struct TcpConn {
    id: String,
    framed_write: FramedWrite<Vec<u8>, WriteHalf<TcpStream>, TcpCodec>,
    tx: UnboundedSender<TunnelMessage>
}

impl TcpConn {
    fn new(id: String, framed_write: FramedWrite<Vec<u8>, WriteHalf<TcpStream>, TcpCodec>, tx: UnboundedSender<TunnelMessage>) -> Self {
        Self { id, framed_write, tx }
    }
}

impl Actor for TcpConn{
    type Context = Context<TcpConn>;
}

impl StreamHandler<Result<Vec<u8>, io::Error>> for TcpConn {
    fn handle(&mut self, item: Result<Vec<u8>, io::Error>, _ctx: &mut Self::Context) {
        let buffer = item.unwrap();
        let message = TunnelMessage::new(self.id.clone(), buffer);
        self.tx.send(message).unwrap();
    }
}

impl WriteHandler<io::Error> for TcpConn {}

impl Handler<TunnelMessage> for TcpConn {
    type Result = ();

    fn handle(&mut self, msg: TunnelMessage, _ctx: &mut Self::Context) {
        self.framed_write.write(msg.data)
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Stop {}

impl Handler<Stop> for TcpConn {
    type Result = ();

    fn handle(&mut self, _msg: Stop, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}

pub async fn tcp_connect(port: u16, id: String, tx: UnboundedSender<TunnelMessage>) -> Addr<TcpConn> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (read, write) = split(stream);
    TcpConn::create(|ctx| {
        let framed_read = FramedRead::new(read, TcpCodec{});
        TcpConn::add_stream(framed_read, ctx);
        let framed_write = FramedWrite::new(write, TcpCodec{}, ctx);
        TcpConn::new(id, framed_write, tx)
    })
}