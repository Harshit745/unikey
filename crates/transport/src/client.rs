use anyhow::Result;
use protocol::{Hello, MessageType, Packet, PacketCodec};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
};

pub struct TransportClient;

impl TransportClient {
    pub async fn connect(
        address: &str,
        port: u16,
        hello: Hello,
    ) -> Result<TcpStream> {
        let mut stream =
            TcpStream::connect(format!("{}:{}", address, port)).await?;

        println!("🔗 Connected to {}:{}", address, port);

        let packet = Packet {
            version: 1,
            message_type: MessageType::Hello,
            payload: serde_json::to_vec(&hello)?,
        };

        let bytes = PacketCodec::encode(&packet);

        stream.write_all(&bytes).await?;

        println!("📤 Hello sent");

        Ok(stream)
    }
}