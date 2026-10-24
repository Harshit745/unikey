//! Discard superseded cursor positions already buffered in a FIFO, without
//! adding a timer, crossing a peer/lease boundary, or discarding discrete input.
use protocol::{MessageType, Packet};
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Deserialize)]
struct CursorMotion {
    cursor: String,
    lease: Uuid,
    event: Motion,
}
#[derive(Deserialize)]
enum Motion {
    Move { x: f64, y: f64 },
}
fn lease(packet: &Packet) -> Option<Uuid> {
    if packet.version != 1 || packet.message_type != MessageType::InputMouseEvent {
        return None;
    }
    let motion: CursorMotion = serde_json::from_slice(&packet.payload).ok()?;
    let Motion::Move { x, y } = motion.event;
    (motion.cursor == "mouse" && x.is_finite() && y.is_finite()).then_some(motion.lease)
}

pub trait MotionItem {
    fn motion_key(&self) -> Option<(Uuid, Uuid)>;
}
impl MotionItem for Packet {
    fn motion_key(&self) -> Option<(Uuid, Uuid)> {
        // An outbound queue belongs to exactly one authenticated TCP session.
        lease(self).map(|lease| (Uuid::nil(), lease))
    }
}
impl MotionItem for (Uuid, Packet) {
    fn motion_key(&self) -> Option<(Uuid, Uuid)> {
        lease(&self.1).map(|lease| (self.0, lease))
    }
}

pub struct LatestMotionReceiver<T> {
    rx: mpsc::Receiver<T>,
    pending: Option<T>,
}
impl<T: MotionItem> LatestMotionReceiver<T> {
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx, pending: None }
    }
    pub async fn recv(&mut self) -> Option<T> {
        let mut current = match self.pending.take() {
            Some(item) => item,
            None => self.rx.recv().await?,
        };
        if let Some(key) = current.motion_key() {
            // Bound work per call even if a producer continuously refills it.
            for _ in 0..255 {
                let Ok(next) = self.rx.try_recv() else { break };
                if next.motion_key() == Some(key) {
                    current = next;
                } else {
                    self.pending = Some(next);
                    break;
                }
            }
        }
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet(lease: Uuid, event: serde_json::Value) -> Packet {
        Packet {
            version: 1,
            message_type: MessageType::InputMouseEvent,
            payload: serde_json::to_vec(
                &serde_json::json!({"cursor":"mouse","lease":lease,"event":event}),
            )
            .unwrap(),
        }
    }
    fn movement(lease: Uuid, x: u32) -> Packet {
        packet(lease, serde_json::json!({"Move":{"x":x,"y":1}}))
    }
    #[tokio::test]
    async fn buffered_motion_is_latest_only_but_clicks_and_release_keep_order() {
        let (tx, rx) = mpsc::channel(256);
        let mut rx = LatestMotionReceiver::new(rx);
        let lease = Uuid::new_v4();
        for x in 0..100 {
            tx.send(movement(lease, x)).await.unwrap();
        }
        let down = packet(
            lease,
            serde_json::json!({"ButtonDown":{"button":0,"x":99,"y":1}}),
        );
        tx.send(down.clone()).await.unwrap();
        for x in 100..200 {
            tx.send(movement(lease, x)).await.unwrap();
        }
        let up = packet(
            lease,
            serde_json::json!({"ButtonUp":{"button":0,"x":199,"y":1}}),
        );
        tx.send(up.clone()).await.unwrap();
        let release = Packet {
            version: 1,
            message_type: MessageType::InputRelease,
            payload: vec![],
        };
        tx.send(release.clone()).await.unwrap();
        drop(tx);
        assert_eq!(
            rx.recv().await.unwrap().payload,
            movement(lease, 99).payload
        );
        assert_eq!(rx.recv().await.unwrap().payload, down.payload);
        assert_eq!(
            rx.recv().await.unwrap().payload,
            movement(lease, 199).payload
        );
        assert_eq!(rx.recv().await.unwrap().payload, up.payload);
        assert_eq!(rx.recv().await.unwrap().message_type, release.message_type);
        assert!(rx.recv().await.is_none());
    }
    #[tokio::test]
    async fn inbound_peer_and_lease_boundaries_are_never_crossed() {
        let (tx, rx) = mpsc::channel(8);
        let mut rx = LatestMotionReceiver::new(rx);
        let (a, b, one, two) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        for (peer, lease, x) in [(a, one, 1), (b, one, 2), (b, two, 3)] {
            tx.send((peer, movement(lease, x))).await.unwrap();
        }
        drop(tx);
        assert_eq!(rx.recv().await.unwrap().0, a);
        assert_eq!(rx.recv().await.unwrap().1.payload, movement(one, 2).payload);
        assert_eq!(rx.recv().await.unwrap().1.payload, movement(two, 3).payload);
        assert!(rx.recv().await.is_none());
    }
    #[test]
    fn scrolling_legacy_invalid_and_other_messages_are_barriers() {
        let id = Uuid::new_v4();
        assert!(
            packet(id, serde_json::json!({"Scroll":{"delta_x":0,"delta_y":1}}))
                .motion_key()
                .is_none()
        );
        assert!(
            Packet {
                version: 1,
                message_type: MessageType::InputMouseEvent,
                payload: b"invalid".to_vec()
            }
            .motion_key()
            .is_none()
        );
        let mut p = movement(id, 1);
        p.message_type = MessageType::FileChunk;
        assert!(p.motion_key().is_none());
    }
}
