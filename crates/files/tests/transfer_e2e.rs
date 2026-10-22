//! End-to-end file transfer test.
//!
//! Spins up two real transport peers over a local TCP pair, wires a
//! `FileTransferService` on each side (Alice sends, Bob receives and routes
//! inbound packets exactly like the daemon router), then verifies the file
//! lands in Bob's download directory byte-for-byte and the transfer is
//! reported complete.

use std::path::PathBuf;
use std::time::Duration;

use crypto::IdentityKey;
use device::{DeviceIdentity, DeviceType, Platform};
use files::{FileTransferService, TransferStatus};
use protocol::Packet;
use services::Service;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
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

/// Unique temporary directory for a test (tests run in parallel).
fn temp_dir(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "unikey-files-e2e-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).expect("create temp base");
    base
}

#[tokio::test]
async fn sends_a_file_over_the_wire_end_to_end() {
    // ── Transport pair ─────────────────────────────────────────────────────
    let (alice_identity, alice_key) = test_identity("alice");
    let (bob_identity, bob_key) = test_identity("bob");
    let alice_sessions = SessionManager::new();
    let bob_sessions = SessionManager::new();
    let (alice_inbound_tx, mut alice_inbound_rx) =
        tokio::sync::mpsc::channel::<(uuid::Uuid, Packet)>(256);
    let (bob_inbound_tx, mut bob_inbound_rx) =
        tokio::sync::mpsc::channel::<(uuid::Uuid, Packet)>(256);
    let alice_conn = ConnectionManager::new(
        alice_identity.clone(),
        alice_key,
        alice_sessions.clone(),
        Some(alice_inbound_tx),
    );
    let bob_conn = ConnectionManager::new(
        bob_identity.clone(),
        bob_key,
        bob_sessions.clone(),
        Some(bob_inbound_tx),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let bob_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        bob_conn
            .handle_incoming(stream)
            .await
            .expect("bob handshake");
    });
    alice_conn
        .connect_to_peer(addr)
        .await
        .expect("alice handshake");
    bob_handle.await.expect("bob task");
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(alice_sessions.has_session(&bob_identity.id).await);
    assert!(bob_sessions.has_session(&alice_identity.id).await);

    // ── Services on both sides ─────────────────────────────────────────────
    let root = temp_dir("happy");
    let bob_downloads = root.join("downloads");
    let alice_files = std::sync::Arc::new(FileTransferService::new());
    let bob_files = std::sync::Arc::new(FileTransferService::with_download_dir(
        bob_downloads.clone(),
    ));

    let shutdown = CancellationToken::new();
    alice_files
        .start(services::ServiceContext::new(
            alice_identity.clone(),
            alice_sessions.clone(),
            shutdown.clone(),
        ))
        .await
        .expect("alice service start");
    bob_files
        .start(services::ServiceContext::new(
            bob_identity.clone(),
            bob_sessions.clone(),
            shutdown.clone(),
        ))
        .await
        .expect("bob service start");

    // Route every inbound packet on Bob to his file service (daemon router
    // behavior: untrusted gate omitted — these peers are already trusted).
    let router_bob = bob_files.clone();
    let bob_router = tokio::spawn(async move {
        while let Some((peer_id, packet)) = bob_inbound_rx.recv().await {
            if let Err(e) = router_bob
                .handle_packet(&peer_id, packet.message_type, &packet.payload)
                .await
            {
                eprintln!("[TEST:router] error: {e}");
            }
        }
    });
    // Route Bob's FileAck back to Alice as the real daemon does. Without this
    // reverse route a sender must never report success.
    let router_alice = alice_files.clone();
    let alice_router = tokio::spawn(async move {
        while let Some((peer_id, packet)) = alice_inbound_rx.recv().await {
            if let Err(e) = router_alice
                .handle_packet(&peer_id, packet.message_type, &packet.payload)
                .await
            {
                eprintln!("[TEST:alice-router] error: {e}");
            }
        }
    });

    // ── Send a real file ───────────────────────────────────────────────────
    let source = root.join("payload.bin");
    // ~350 KiB — spans multiple 64 KiB chunks.
    let content: Vec<u8> = (0..350_000u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(&source, &content).expect("write source");

    let transfer_id = alice_files
        .send_file(&bob_identity.id, &source)
        .await
        .expect("send_file accepted");

    // ── Wait for Bob to complete ───────────────────────────────────────────
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut final_status = None;
    while tokio::time::Instant::now() < deadline {
        if let Some(info) = bob_files.get(transfer_id).await {
            final_status = Some(info.status);
            if matches!(
                info.status,
                TransferStatus::Completed | TransferStatus::Verified | TransferStatus::Failed
            ) {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        matches!(
            final_status,
            Some(TransferStatus::Completed) | Some(TransferStatus::Verified)
        ),
        "expected completed/verified, got {final_status:?}"
    );

    // The file arrived intact at its final name.
    let received_path = bob_downloads.join("payload.bin");
    assert!(received_path.is_file(), "file written to downloads dir");
    let received = std::fs::read(&received_path).expect("read received");
    assert_eq!(received, content, "payload matches byte-for-byte");

    // Sender also reports it finished.
    let sender_info = alice_files.get(transfer_id).await.expect("sender info");
    assert!(matches!(sender_info.status, TransferStatus::Verified));

    shutdown.cancel();
    bob_router.abort();
    alice_router.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn receiver_rejects_oversized_metadata() {
    use files::transfer::{FileMetadata, MAX_FILE_SIZE};

    let bob_downloads = temp_dir("oversize").join("downloads");
    let bob_files = FileTransferService::with_download_dir(bob_downloads.clone());

    // Not started — handle_packet has no session to reply with, so it must
    // still reject the metadata (returns Ok, transfer never registered).
    let peer = uuid::Uuid::new_v4();
    let meta = FileMetadata {
        transfer_id: uuid::Uuid::new_v4(),
        filename: "huge.bin".into(),
        total_size: MAX_FILE_SIZE + 1,
        checksum: [0u8; 32],
        total_chunks: 1,
    };
    let payload = serde_json::to_vec(&meta).unwrap();
    bob_files
        .handle_packet(&peer, protocol::MessageType::FileMetadata, &payload)
        .await
        .expect("rejects without erroring");

    assert!(
        bob_files.get(meta.transfer_id).await.is_none(),
        "not registered"
    );
}
