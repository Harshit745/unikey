use bytes::{BufMut, BytesMut};

use crate::MessageType;

#[derive(Debug, Clone)]
pub struct Packet {
    pub version: u8,

    pub message_type: MessageType,

    pub payload: Vec<u8>,
}

impl Packet {
    pub fn encode(&self) -> BytesMut {
        let mut buffer = BytesMut::new();

        buffer.put_u8(self.version);

        buffer.put_u8(self.message_type as u8);

        buffer.put_u32(self.payload.len() as u32);

        buffer.extend_from_slice(&self.payload);

        buffer
    }
}