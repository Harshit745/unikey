//! End-to-end transport integration tests.
//!
//! These spin up two real `ConnectionManager`s on a local TCP pair and verify
//! the full pipeline: handshake → encrypted session → packet round-trip.

use std::time::Duration;

use crypto::IdentityKey;
use device::{DeviceIdentity, DeviceType, Platform};
use protocol::{MessageType, Packet, PacketCodec};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use transport::connection::ConnectionManager;
use transport::session::SessionManager;

fn test_identity(name: &str) -> (DeviceIdentity, IdentityKey) {
    let key = IdentityKey::generate();
    let identity = DeviceIdentity {
        id: uuid::Uuid::new_v4(),
        name: name.to_string(),
        platform: Platform::MacOS,
        device_type: DeviceType::Desktop,
        created_at: chrono::Utc::now(),
        protocol_version: 1,
        public_key_path: String::new(),
        private_key_path: String::new(),
    };
    (identity, key)
}

fn manager(name: &str) -> (ConnectionManager, SessionManager, DeviceIdentity) {
    let (identity, key) = test_identity(name);
    let sessions = SessionManager::new();
    let conn = ConnectionManager::new(identity.clone(), key, sessions.clone(), None);
    (conn, sessions, identity)
}

/// A real encrypted TCP stream, not a device/OS injection benchmark. Run with
/// `cargo test -p transport --test secure_session cursor_stream_latency -- --ignored --nocapture`.
#[tokio::test]
#[ignore = "reports host-dependent loopback timing; not a CI latency assertion"]
async fn cursor_stream_latency() {
    let (source, sessions, _) = manager("cursor-source");
    let (identity, key) = test_identity("cursor-receiver");
    let (tx, mut rx) = tokio::sync::mpsc::channel(512);
    let receiver = ConnectionManager::new(identity.clone(), key, SessionManager::new(), Some(tx));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accepted = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        receiver.handle_incoming(stream).await.unwrap();
    });
    source.connect_to_peer(addr).await.unwrap();
    accepted.await.unwrap();
    let sender = sessions.get_session_sender(&identity.id).await.unwrap();
    let started = std::time::Instant::now();
    let producer = tokio::spawn(async move {
        let mut clock = tokio::time::interval(Duration::from_millis(1));
        clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        for _ in 0..200 {
            clock.tick().await;
            sender.send(Packet {
                version: 1,
                message_type: MessageType::InputMouseEvent,
                payload: (started.elapsed().as_micros() as u64).to_be_bytes().to_vec(),
            }).await.unwrap();
        }
    });
    let mut micros = Vec::new();
    for _ in 0..200 {
        let (_, packet) = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await.unwrap().unwrap();
        let sent = u64::from_be_bytes(packet.payload.try_into().unwrap());
        micros.push(started.elapsed().as_micros() as u64 - sent);
    }
    producer.await.unwrap();
    micros.sort_unstable();
    eprintln!("encrypted loopback, 200 small packets at 1ms: p50={}us p95={}us max={}us", micros[100], micros[190], micros[199]);
}

#[tokio::test]
async fn input_sessions_disable_nagle_on_both_sockets() {
    let (alice, _, _) = manager("alice-low-latency");
    let (bob, _, _) = manager("bob-low-latency");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = tokio::net::TcpStream::connect(listener.local_addr().unwrap()).await.unwrap().into_std().unwrap();
    let (b, _) = listener.accept().await.unwrap();
    let b = b.into_std().unwrap();
    let observe_a = a.try_clone().unwrap();
    let observe_b = b.try_clone().unwrap();
    let (a_result, b_result) = tokio::join!(
        alice.handle_incoming(tokio::net::TcpStream::from_std(a).unwrap()),
        bob.handle_incoming(tokio::net::TcpStream::from_std(b).unwrap())
    );
    a_result.unwrap();
    b_result.unwrap();
    assert!(observe_a.nodelay().unwrap());
    assert!(observe_b.nodelay().unwrap());
}

#[tokio::test]
async fn encrypted_cursor_burst_keeps_latest_positions_and_button_order() {
    let (source, sessions, _) = manager("burst-source");
    let (identity, key) = test_identity("burst-receiver");
    let (tx, rx) = tokio::sync::mpsc::channel(256);
    let mut rx = transport::motion::LatestMotionReceiver::new(rx);
    let receiver = ConnectionManager::new(identity.clone(), key, SessionManager::new(), Some(tx));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accepted = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        receiver.handle_incoming(stream).await.unwrap();
    });
    source.connect_to_peer(addr).await.unwrap();
    accepted.await.unwrap();
    let sender = sessions.get_session_sender(&identity.id).await.unwrap();
    let lease = uuid::Uuid::new_v4();
    let make = |event| Packet {
        version: 1, message_type: MessageType::InputMouseEvent,
        payload: serde_json::to_vec(&serde_json::json!({"cursor":"mouse","lease":lease,"event":event})).unwrap(),
    };
    for x in 0..100 {
        sender.try_send(make(serde_json::json!({"Move":{"x":x,"y":10}}))).unwrap();
    }
    sender.try_send(make(serde_json::json!({"ButtonDown":{"button":0,"x":99,"y":10}}))).unwrap();
    for x in 100..200 {
        sender.try_send(make(serde_json::json!({"Move":{"x":x,"y":10}}))).unwrap();
    }
    sender.try_send(make(serde_json::json!({"ButtonUp":{"button":0,"x":199,"y":10}}))).unwrap();
    let mut last_x = None;
    let mut pressed = false;
    loop {
        let (_, packet) = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await.unwrap().unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&packet.payload).unwrap();
        let event = &payload["event"];
        if let Some(x) = event["Move"]["x"].as_u64() { last_x = Some(x); }
        if event.get("ButtonDown").is_some() {
            assert_eq!(last_x, Some(99));
            assert!(!pressed);
            pressed = true;
        }
        if event.get("ButtonUp").is_some() {
            assert!(pressed);
            assert_eq!(last_x, Some(199));
            break;
        }
    }
}

#[tokio::test]
async fn two_peers_establish_encrypted_session() {
    let (alice_conn, alice_sessions, alice_id) = manager("alice");
    let (bob_conn, bob_sessions, bob_id) = manager("bob");

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    // Bob accepts the inbound connection and runs the handshake.
    let bob_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        bob_conn.handle_incoming(stream).await.expect("bob handshake");
    });

    // Alice initiates.
    alice_conn
        .connect_to_peer(addr)
        .await
        .expect("alice handshake");

    bob_handle.await.expect("bob task");

    // Wait for both session tables to settle (handshake completes async).
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Both sides register each other's device id — and not their own.
    assert!(alice_sessions.has_session(&bob_id.id).await, "alice sees bob");
    assert!(bob_sessions.has_session(&alice_id.id).await, "bob sees alice");
    assert!(!alice_sessions.has_session(&alice_id.id).await, "no self-session");
    assert_eq!(alice_sessions.session_count().await, 1);
    assert_eq!(bob_sessions.session_count().await, 1);

    // Session metadata is exported.
    let infos = alice_sessions.list_sessions().await;
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].peer_id, bob_id.id);
    assert!(!infos[0].peer_name.is_empty());
}

#[tokio::test]
async fn handshake_rejects_tampered_signature() {
    // Build a bogus peer: valid Hello shape but an invalid signature.
    let key = IdentityKey::generate();
    let identity = test_identity("malicious").0;
    let hello = protocol::Hello {
        protocol_version: 1,
        device_id: identity.id,
        device_name: identity.name,
        platform: "Desktop".to_string(),
        file_chunk_binary: false,
        ed25519_public_key: key.public_key(),
        x25519_public_key: [0u8; 32],
        timestamp: 0,
        signature: [0u8; 64], // invalid signature
    };

    // Serialize it as a raw Hello packet (the transport reads these on the wire).
    let payload = serde_json::to_vec(&hello).unwrap();
    let _packet = Packet {
        version: 1,
        message_type: MessageType::Hello,
        payload,
    };

    // We can't easily drive a full handshake with a bad peer, so verify the
    // signature-check primitive rejects it directly — that's the same check
    // `HandshakeContext` performs.
    let verified = crypto::verify_handshake(
        &hello.ed25519_public_key,
        &hello.device_id,
        &hello.x25519_public_key,
        hello.timestamp,
        &hello.signature,
    );
    assert!(!verified, "a zeroed signature must fail verification");
}

#[tokio::test]
async fn handshake_rejects_legacy_unauthenticated_hello() {
    let (client, sessions, _) = manager("client");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");

        // Consume the client's signed Hello before sending an old zero-key
        // Hello. This drives the real transport handshake, not just the crypto
        // primitive in isolation.
        let mut header = [0u8; 6];
        stream.read_exact(&mut header).await.expect("hello header");
        let payload_len = u32::from_be_bytes(header[2..6].try_into().unwrap()) as usize;
        let mut payload = vec![0; payload_len];
        stream.read_exact(&mut payload).await.expect("hello payload");

        let hello = protocol::Hello::default();
        let packet = Packet {
            version: 1,
            message_type: MessageType::Hello,
            payload: serde_json::to_vec(&hello).unwrap(),
        };
        stream
            .write_all(&PacketCodec::encode(&packet))
            .await
            .expect("legacy hello write");
    });

    let error = client
        .connect_to_peer(addr)
        .await
        .expect_err("unauthenticated Hello must be rejected");
    assert!(error.to_string().contains("missing required identity"));
    assert_eq!(sessions.session_count().await, 0);
    peer.await.expect("peer task");
}

#[tokio::test]
async fn encrypted_packet_round_trip() {
    // Set up two connected managers with an inbound channel on the receiver.
    let (alice_conn, alice_sessions, alice_id) = manager("alice");
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(uuid::Uuid, Packet)>(16);
    let (bob_identity, bob_key) = test_identity("bob");
    let bob_sessions = SessionManager::new();
    let bob_conn = ConnectionManager::new(bob_identity.clone(), bob_key, bob_sessions.clone(), Some(tx));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let bob_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        bob_conn.handle_incoming(stream).await.expect("bob handshake");
    });

    alice_conn.connect_to_peer(addr).await.expect("alice handshake");
    bob_handle.await.expect("bob task");

    // Wait for session registration + connection task spin-up.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Both sides register each other.
    assert!(alice_sessions.has_session(&bob_identity.id).await, "alice→bob session");
    assert!(bob_sessions.has_session(&alice_id.id).await, "bob→alice session");

    // Alice sends a Clipboard packet to bob.
    let payload = serde_json::to_vec(&serde_json::json!({"text": "hello from alice"})).unwrap();
    let packet = Packet {
        version: 1,
        message_type: MessageType::Clipboard,
        payload,
    };
    let sender = alice_sessions
        .get_session_sender(&bob_identity.id)
        .await
        .expect("alice has bob's session");
    sender.send(packet).await.expect("packet sent");

    // Bob's inbound channel should deliver it, tagged with alice's peer id.
    let (peer_id, received) = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("bob receives packet")
        .expect("channel not closed");

    assert_eq!(peer_id, alice_id.id, "inbound tagged with sender's peer id");
    assert_eq!(received.message_type, MessageType::Clipboard);
    let decoded: serde_json::Value = serde_json::from_slice(&received.payload).unwrap();
    assert_eq!(decoded["text"], "hello from alice");
}

#[tokio::test]
async fn packet_codec_round_trip() {
    let packet = Packet {
        version: 1,
        message_type: MessageType::FileMetadata,
        payload: b"\x00\x01\x02".to_vec(),
    };
    let bytes = PacketCodec::encode(&packet);
    let decoded = PacketCodec::decode(&bytes).unwrap();
    assert_eq!(decoded.version, packet.version);
    assert_eq!(decoded.message_type, packet.message_type);
    assert_eq!(decoded.payload, packet.payload);
}

#[tokio::test]
async fn graceful_disconnect_closes_session() {
    let (alice_conn, alice_sessions, _) = manager("alice");
    let (bob_identity, bob_key) = test_identity("bob");
    let bob_sessions = SessionManager::new();
    let bob_conn = ConnectionManager::new(bob_identity.clone(), bob_key, bob_sessions.clone(), None);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let bob_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let _ = bob_conn.handle_incoming(stream).await;
    });

    alice_conn.connect_to_peer(addr).await.expect("alice handshake");
    bob_handle.await.expect("bob task");
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(
        alice_sessions.has_session(&bob_identity.id).await,
        "session exists before disconnect"
    );

    alice_conn.disconnect(&bob_identity.id).await;

    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !alice_sessions.has_session(&bob_identity.id).await,
        "session removed after disconnect"
    );
    assert_eq!(alice_sessions.session_count().await, 0);
}
