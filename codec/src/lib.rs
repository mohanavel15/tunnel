use std::io;
use bytes::BufMut;
use actix_codec::{Decoder, Encoder};

pub struct TcpCodec {}

impl Encoder<Vec<u8>> for TcpCodec {
    type Error = io::Error;
    fn encode(&mut self, item: Vec<u8>, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        let buffer = item.as_slice();
        dst.put_slice(buffer);
        Ok(())
    }
}

impl Decoder for TcpCodec {
    type Item = Vec<u8>;
    type Error = io::Error;
    
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let buffer = src.to_vec();
        src.clear();
        if buffer.is_empty() {
            Ok(None)
        } else {
            Ok(Some(buffer))
        }
    }
}