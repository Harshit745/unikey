//! Latest-position queue: collapse only adjacent absolute mouse moves for the
//! same peer/lease. Clicks, scrolling and ownership changes are ordering barriers.
use crate::cursor::{Outbound, Wire};
use crate::event::MouseEvent;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

const CAPACITY: usize = 256;

#[derive(Debug, Default)]
struct Shared {
    events: Mutex<VecDeque<Outbound>>,
    ready: Notify,
}

#[derive(Debug, Clone)]
pub(crate) struct OutboundSender(Arc<Shared>);

#[derive(Debug)]
pub struct OutboundReceiver(Arc<Shared>);

pub(crate) fn channel() -> (OutboundSender, OutboundReceiver) {
    let shared = Arc::new(Shared::default());
    (OutboundSender(shared.clone()), OutboundReceiver(shared))
}

impl OutboundSender {
    pub fn try_send(
        &self,
        event: Outbound,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<Outbound>> {
        let mut events = self.0.events.lock().unwrap();
        let replace = events.back().is_some_and(|previous| {
            previous.peer == event.peer
                && matches!(
                    (&previous.wire, &event.wire),
                    (Wire::Mouse { lease: a, event: MouseEvent::Move { .. } },
                     Wire::Mouse { lease: b, event: MouseEvent::Move { .. } }) if a == b
                )
        });
        if replace {
            *events.back_mut().unwrap() = event;
        } else {
            if events.len() >= CAPACITY {
                return Err(tokio::sync::mpsc::error::TrySendError::Full(event));
            }
            events.push_back(event);
        }
        drop(events);
        self.0.ready.notify_one();
        Ok(())
    }
}

impl OutboundReceiver {
    /// Only one receiver exists. Notify's stored permit prevents a lost wakeup
    /// between checking the queue and awaiting. Service cancellation owns exit.
    pub async fn recv(&mut self) -> Outbound {
        loop {
            let ready = self.0.ready.notified();
            if let Some(event) = self.0.events.lock().unwrap().pop_front() {
                return event;
            }
            ready.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    fn movement(peer: Uuid, lease: Uuid, x: f64) -> Outbound {
        Outbound {
            peer,
            wire: Wire::Mouse {
                lease,
                event: MouseEvent::Move { x, y: 10. },
            },
        }
    }
    #[tokio::test]
    async fn ten_thousand_pending_moves_keep_only_the_latest_position() {
        let (tx, mut rx) = channel();
        let (peer, lease) = (Uuid::new_v4(), Uuid::new_v4());
        for x in 0..10_000 {
            tx.try_send(movement(peer, lease, x as f64)).unwrap();
        }
        assert_eq!(tx.0.events.lock().unwrap().len(), 1);
        assert!(matches!(
            rx.recv().await.wire,
            Wire::Mouse {
                event: MouseEvent::Move { x: 9999., .. },
                ..
            }
        ));
    }
    #[tokio::test]
    async fn clicks_scroll_and_release_are_never_coalesced_or_reordered() {
        let (tx, mut rx) = channel();
        let (peer, lease) = (Uuid::new_v4(), Uuid::new_v4());
        tx.try_send(movement(peer, lease, 1.)).unwrap();
        tx.try_send(movement(peer, lease, 2.)).unwrap();
        tx.try_send(Outbound {
            peer,
            wire: Wire::Mouse {
                lease,
                event: MouseEvent::ButtonDown {
                    button: 0,
                    x: 2.,
                    y: 10.,
                },
            },
        })
        .unwrap();
        tx.try_send(movement(peer, lease, 3.)).unwrap();
        tx.try_send(movement(peer, lease, 4.)).unwrap();
        tx.try_send(Outbound {
            peer,
            wire: Wire::Mouse {
                lease,
                event: MouseEvent::Scroll {
                    delta_x: 0.,
                    delta_y: 1.,
                },
            },
        })
        .unwrap();
        tx.try_send(Outbound {
            peer,
            wire: Wire::Mouse {
                lease,
                event: MouseEvent::ButtonUp {
                    button: 0,
                    x: 4.,
                    y: 10.,
                },
            },
        })
        .unwrap();
        tx.try_send(Outbound {
            peer,
            wire: Wire::Release { lease, y: 0.5 },
        })
        .unwrap();
        assert!(matches!(
            rx.recv().await.wire,
            Wire::Mouse {
                event: MouseEvent::Move { x: 2., .. },
                ..
            }
        ));
        assert!(matches!(
            rx.recv().await.wire,
            Wire::Mouse {
                event: MouseEvent::ButtonDown { .. },
                ..
            }
        ));
        assert!(matches!(
            rx.recv().await.wire,
            Wire::Mouse {
                event: MouseEvent::Move { x: 4., .. },
                ..
            }
        ));
        assert!(matches!(
            rx.recv().await.wire,
            Wire::Mouse {
                event: MouseEvent::Scroll { .. },
                ..
            }
        ));
        assert!(matches!(
            rx.recv().await.wire,
            Wire::Mouse {
                event: MouseEvent::ButtonUp { .. },
                ..
            }
        ));
        assert!(matches!(rx.recv().await.wire, Wire::Release { .. }));
    }
    #[test]
    fn peer_and_lease_changes_are_barriers_and_discrete_overflow_fails() {
        let (tx, _) = channel();
        let peer = Uuid::new_v4();
        let lease = Uuid::new_v4();
        tx.try_send(movement(peer, lease, 1.)).unwrap();
        tx.try_send(movement(peer, Uuid::new_v4(), 2.)).unwrap();
        tx.try_send(movement(Uuid::new_v4(), lease, 3.)).unwrap();
        assert_eq!(tx.0.events.lock().unwrap().len(), 3);
        for _ in 3..CAPACITY {
            tx.try_send(Outbound {
                peer,
                wire: Wire::Release { lease, y: 0.5 },
            })
            .unwrap();
        }
        assert!(
            tx.try_send(Outbound {
                peer,
                wire: Wire::Release { lease, y: 0.5 }
            })
            .is_err()
        );
    }
    #[tokio::test]
    async fn idle_receiver_wakes_and_cancelled_wait_does_not_lose_events() {
        let (tx, mut rx) = channel();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(1), rx.recv())
                .await
                .is_err()
        );
        let event = movement(Uuid::new_v4(), Uuid::new_v4(), 1.);
        let sender = tokio::spawn(async move {
            tokio::task::yield_now().await;
            tx.try_send(event).unwrap();
        });
        tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap();
        sender.await.unwrap();
    }
}
