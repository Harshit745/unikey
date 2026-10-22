use anyhow::{bail, Result};
use bytes::BytesMut;

use crate::{MessageType, Packet};

pub struct PacketCodec;

impl PacketCodec {
    pub fn decode(data: &[u8]) -> Result<Packet> {
        if data.len() < 6 {
            bail!("packet too small");
        }

        let version = data[0];

        let message_type =
            MessageType::try_from(data[1]).map_err(|_| anyhow::anyhow!("invalid message"))?;

        let payload_length =
            u32::from_be_bytes([data[2], data[3], data[4], data[5]]) as usize;

        if data.len() < payload_length + 6 {
            bail!("incomplete packet");
        }

        Ok(Packet {
            version,
            message_type,
            payload: data[6..6 + payload_length].to_vec(),
        })
    }

    pub fn encode(packet: &Packet) -> BytesMut {
        packet.encode()
    }
}