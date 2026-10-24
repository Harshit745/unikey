use super::*;
use crate::cursor::Screen;
#[derive(Debug, Default)]
struct RecordingInjector(Mutex<Vec<InputAction>>);
impl InputInjector for RecordingInjector {
    fn inject(&self, action: &InputAction) -> Result<()> {
        self.0.lock().unwrap().push(action.clone());
        Ok(())
    }
}
struct Fixture {
    service: InputService,
    injector: Arc<RecordingInjector>,
    peer: Uuid,
    outgoing: mpsc::Receiver<Packet>,
    shutdown: tokio_util::sync::CancellationToken,
    root: std::path::PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.shutdown.cancel();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
async fn fixture() -> Fixture {
    let peer = Uuid::new_v4();
    let root = std::env::temp_dir().join(format!("unikey-cursor-test-{}", Uuid::new_v4()));
    let pairing = Arc::new(pairing::PairingService::with_store_root(Some(&root)).unwrap());
    let sessions = transport::session::SessionManager::new();
    let (tx, outgoing) = mpsc::channel(32);
    sessions
        .add_session(transport::session::Session {
            connection_id: Uuid::new_v4(),
            peer_id: peer,
            peer_name: "test-peer".into(),
            platform: "Windows".into(),
            peer_public_key: [9; 32],
            file_chunk_binary: false,
            tx,
            session_keys: crypto::SessionKeys {
                send_key: [1; 32],
                recv_key: [2; 32],
            },
            inbound_replay: crypto::ReplayGuard::new(),
            outbound_counter: 0,
            last_pong_received: Instant::now(),
            last_ping_sent: None,
            rtt_ms: None,
            last_activity: Instant::now(),
        })
        .await;
    let identity = device::DeviceIdentity {
        id: Uuid::new_v4(),
        name: "test-local".into(),
        platform: device::Platform::MacOS,
        device_type: device::DeviceType::Desktop,
        created_at: chrono::Utc::now(),
        protocol_version: 1,
        public_key_path: String::new(),
        private_key_path: String::new(),
    };
    let shutdown = tokio_util::sync::CancellationToken::new();
    let ctx = ServiceContext::new(identity, sessions, shutdown.clone());
    pairing.start(ctx.clone()).await.unwrap();
    pairing
        .complete_pairing(&Uuid::new_v4(), peer, "test-peer", "Windows", &[9; 32])
        .await
        .unwrap();
    let (cursor, rx) = CursorController::new();
    let injector = Arc::new(RecordingInjector::default());
    let service = InputService {
        injector: injector.clone(),
        cursor,
        outbound: Arc::new(Mutex::new(Some(rx))),
        context: Arc::new(Mutex::new(Some(ctx))),
        pairing: Some(pairing),
    };
    {
        let mut s = service.cursor.state.lock().unwrap();
        s.screen = Some(Screen {
            x: 0.,
            y: 0.,
            width: 1920.,
            height: 1080.,
        });
    }
    Fixture {
        service,
        injector,
        peer,
        outgoing,
        shutdown,
        root,
    }
}
async fn deliver(f: &Fixture, wire: Wire) -> Result<()> {
    f.service
        .handle_packet(&f.peer, MessageType::InputGrab, &serde_json::to_vec(&wire)?)
        .await
}
#[tokio::test]
async fn receiver_requires_opt_in_then_acknowledges_actual_geometry() {
    let mut f = fixture().await;
    let request = Uuid::new_v4();
    assert!(deliver(&f, Wire::Probe { request }).await.is_err());
    assert!(f.outgoing.try_recv().is_err());
    assert!(f.injector.0.lock().unwrap().is_empty());
    f.service.cursor.state.lock().unwrap().receiving = true;
    deliver(&f, Wire::Probe { request }).await.unwrap();
    let packet = f.outgoing.recv().await.unwrap();
    assert!(
        matches!(serde_json::from_slice::<Wire>(&packet.payload).unwrap(),Wire::Ready{request:r,screen} if r==request && screen.width==1920.)
    );
}
#[tokio::test]
async fn ownership_rejects_wrong_peer_lease_and_out_of_bounds_coordinates() {
    let f = fixture().await;
    f.service.cursor.state.lock().unwrap().receiving = true;
    let lease = Uuid::new_v4();
    deliver(
        &f,
        Wire::Enter {
            lease,
            edge: Edge::Right,
            y: 0.5,
        },
    )
    .await
    .unwrap();
    let count = f.injector.0.lock().unwrap().len();
    assert!(
        deliver(
            &f,
            Wire::Mouse {
                lease: Uuid::new_v4(),
                event: MouseEvent::Move { x: 30., y: 40. }
            }
        )
        .await
        .is_err()
    );
    assert!(
        deliver(
            &f,
            Wire::Mouse {
                lease,
                event: MouseEvent::Move { x: 9999., y: 40. }
            }
        )
        .await
        .is_err()
    );
    let data = serde_json::to_vec(&Wire::Mouse {
        lease,
        event: MouseEvent::Move { x: 30., y: 40. },
    })
    .unwrap();
    assert!(
        f.service
            .handle_packet(&Uuid::new_v4(), MessageType::InputMouseEvent, &data)
            .await
            .is_err()
    );
    assert_eq!(f.injector.0.lock().unwrap().len(), count);
    deliver(
        &f,
        Wire::Mouse {
            lease,
            event: MouseEvent::Move { x: 300., y: 400. },
        },
    )
    .await
    .unwrap();
    assert_eq!(f.injector.0.lock().unwrap().len(), count + 1);
}
#[tokio::test]
async fn disconnect_releases_held_buttons_and_ends_ownership() {
    let f = fixture().await;
    f.service.cursor.state.lock().unwrap().receiving = true;
    let lease = Uuid::new_v4();
    deliver(
        &f,
        Wire::Enter {
            lease,
            edge: Edge::Right,
            y: 0.5,
        },
    )
    .await
    .unwrap();
    deliver(
        &f,
        Wire::Mouse {
            lease,
            event: MouseEvent::ButtonDown {
                button: 0,
                x: 20.,
                y: 30.,
            },
        },
    )
    .await
    .unwrap();
    {
        f.service
            .cursor
            .state
            .lock()
            .unwrap()
            .receiver
            .as_mut()
            .unwrap()
            .last_seen = Instant::now() - Duration::from_secs(4);
    }
    f.service.tick().await;
    assert!(f.service.cursor.state.lock().unwrap().receiver.is_none());
    assert!(!f.service.cursor.state.lock().unwrap().receiver_idle);
    assert!(matches!(
        f.injector.0.lock().unwrap().last(),
        Some(InputAction::Mouse(MouseEvent::ButtonUp { button: 0, .. }))
    ));
}

#[tokio::test]
async fn normal_return_hides_old_remote_pointer_until_real_local_input() {
    let f = fixture().await;
    f.service.cursor.state.lock().unwrap().receiving = true;
    let lease = Uuid::new_v4();
    deliver(&f, Wire::Enter { lease, edge: Edge::Right, y: 0.5 }).await.unwrap();
    assert!(!f.service.cursor.state.lock().unwrap().receiver_idle);
    deliver(&f, Wire::Release { lease, y: 0.5 }).await.unwrap();
    assert!(f.service.cursor.state.lock().unwrap().receiver_idle);
    assert_eq!(f.service.status().phase, "local");
    // Synthetic remote packets never call capture; native hardware does.
    assert!(!f.service.cursor.capture(crate::cursor::Captured::Move {
        x: 20., y: 30., dx: 1., dy: 0.,
    }));
    assert!(!f.service.cursor.state.lock().unwrap().receiver_idle);
}

#[tokio::test]
async fn disabling_receiving_and_emergency_stop_always_reveal_local_cursor() {
    let f = fixture().await;
    f.service.cursor.state.lock().unwrap().receiver_idle = true;
    f.service.set_receiving(false).unwrap();
    assert!(!f.service.cursor.state.lock().unwrap().receiver_idle);
    f.service.cursor.state.lock().unwrap().receiver_idle = true;
    f.service.stop_cursor();
    assert!(!f.service.cursor.state.lock().unwrap().receiver_idle);
}
