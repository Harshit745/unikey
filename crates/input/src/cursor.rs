//! Explicit cursor ownership over the existing authenticated transport.
//! Native callbacks never await or perform network I/O. A bounded queue hands
//! work to the async service; saturation immediately returns control locally.
use crate::event::MouseEvent;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

pub const LEASE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Screen {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Screen {
    pub fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.width >= 32.0
            && self.height >= 32.0
            && self.width <= 65536.0
            && self.height <= 65536.0
    }
    fn entry(self, edge: Edge, y: f64) -> (f64, f64) {
        (
            match edge {
                Edge::Right => self.x + 3.0,
                Edge::Left => self.x + self.width - 4.0,
            },
            self.y + y.clamp(0.0, 1.0) * (self.height - 1.0),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cursor", rename_all = "snake_case")]
pub enum Wire {
    Probe { request: Uuid },
    Ready { request: Uuid, screen: Screen },
    Enter { lease: Uuid, edge: Edge, y: f64 },
    Entered { lease: Uuid },
    Mouse { lease: Uuid, event: MouseEvent },
    Release { lease: Uuid, y: f64 },
    Heartbeat { lease: Uuid },
}

#[derive(Debug, Clone, Serialize)]
pub struct CursorStatus {
    pub supported: bool,
    pub receiving_enabled: bool,
    pub capture_running: bool,
    pub phase: String,
    pub peer_id: Option<Uuid>,
    pub edge: Option<Edge>,
    pub error: Option<String>,
    pub screen: Option<Screen>,
}

#[derive(Debug, Clone)]
pub enum Captured {
    Move { x: f64, y: f64, dx: f64, dy: f64 },
    Down(u8),
    Up(u8),
    Scroll(f64, f64),
    Escape,
}

#[derive(Debug, Clone)]
pub struct Outbound {
    pub peer: Uuid,
    pub wire: Wire,
}

#[derive(Debug)]
pub(crate) struct Source {
    pub peer: Uuid,
    pub edge: Edge,
    pub request: Uuid,
    pub remote: Option<Screen>,
    pub lease: Option<Uuid>,
    pub entered: bool,
    pub point: (f64, f64),
    pub last_seen: Instant,
    pub requested: Instant,
}
#[derive(Debug)]
pub(crate) struct Receiver {
    pub peer: Uuid,
    pub lease: Uuid,
    pub last_seen: Instant,
    pub point: (f64, f64),
    pub buttons: Vec<u8>,
}
#[derive(Debug)]
pub(crate) struct State {
    pub receiving: bool,
    pub capture_running: bool,
    pub screen: Option<Screen>,
    pub source: Option<Source>,
    pub receiver: Option<Receiver>,
    pub error: Option<String>,
    pub warp: Option<(f64, f64)>,
    pub cooldown: Instant,
    pub local_buttons: u8,
}

#[derive(Debug, Clone)]
pub struct CursorController {
    pub(crate) state: Arc<Mutex<State>>,
    pub(crate) tx: mpsc::Sender<Outbound>,
}
impl CursorController {
    pub fn new() -> (Self, mpsc::Receiver<Outbound>) {
        let (tx, rx) = mpsc::channel(256);
        (
            Self {
                tx,
                state: Arc::new(Mutex::new(State {
                    receiving: false,
                    capture_running: false,
                    screen: None,
                    source: None,
                    receiver: None,
                    error: None,
                    warp: None,
                    cooldown: Instant::now(),
                    local_buttons: 0,
                })),
            },
            rx,
        )
    }
    pub fn status(&self) -> CursorStatus {
        let s = self.state.lock().unwrap();
        let phase = match &s.source {
            Some(c) if c.entered => "remote",
            Some(c) if c.lease.is_some() => "entering",
            Some(c) if c.remote.is_some() => "armed",
            Some(_) => "checking_peer",
            None if s.receiver.is_some() => "receiving",
            None => "local",
        };
        CursorStatus {
            supported: cfg!(any(target_os = "macos", windows)),
            receiving_enabled: s.receiving,
            capture_running: s.capture_running,
            phase: phase.into(),
            peer_id: s
                .source
                .as_ref()
                .map(|c| c.peer)
                .or_else(|| s.receiver.as_ref().map(|r| r.peer)),
            edge: s.source.as_ref().map(|c| c.edge),
            error: s.error.clone(),
            screen: s.screen,
        }
    }
    pub fn arm(&self, peer: Uuid, edge: Edge) -> Result<()> {
        let mut s = self.state.lock().unwrap();
        if !s.capture_running {
            bail!("Native mouse capture is unavailable. Grant Accessibility access, then retry.");
        }
        if s.receiver.is_some() {
            bail!("Another device is controlling this computer. Stop receiving first.");
        }
        let request = Uuid::new_v4();
        self.tx.try_send(Outbound {
            peer,
            wire: Wire::Probe { request },
        })?;
        s.source = Some(Source {
            peer,
            edge,
            request,
            remote: None,
            lease: None,
            entered: false,
            point: (0.0, 0.0),
            last_seen: Instant::now(),
            requested: Instant::now(),
        });
        s.error = None;
        Ok(())
    }
    pub fn stop(&self, error: Option<String>) {
        let mut s = self.state.lock().unwrap();
        if let Some(c) = s.source.take() {
            if let Some(lease) = c.lease {
                let _ = self.tx.try_send(Outbound {
                    peer: c.peer,
                    wire: Wire::Release { lease, y: 0.5 },
                });
            }
            if let Some(screen) = s.screen {
                let x = match c.edge {
                    Edge::Left => screen.x + 12.0,
                    Edge::Right => screen.x + screen.width - 13.0,
                };
                s.warp = Some((x, screen.y + screen.height / 2.0));
            }
        }
        s.error = error;
        s.cooldown = Instant::now() + Duration::from_millis(500);
    }
    pub fn capture(&self, event: Captured) -> bool {
        if matches!(event, Captured::Escape) {
            let active = {
                let s = self.state.lock().unwrap();
                s.source.is_some() || s.receiver.is_some()
            };
            if !active {
                return false;
            }
            self.stop(None);
            let mut s = self.state.lock().unwrap();
            s.receiving = false;
            if let Some(r) = s.receiver.as_mut() {
                r.last_seen = Instant::now() - LEASE_TIMEOUT;
                let _ = self.tx.try_send(Outbound {
                    peer: r.peer,
                    wire: Wire::Release {
                        lease: r.lease,
                        y: 0.5,
                    },
                });
            }
            return false;
        }
        let mut s = self.state.lock().unwrap();
        let entered = s.source.as_ref().is_some_and(|c| c.entered);
        if !entered {
            match event {
                Captured::Down(b) if b < 3 => s.local_buttons |= 1 << b,
                Captured::Up(b) if b < 3 => s.local_buttons &= !(1 << b),
                _ => {}
            }
        }
        let holding_local_button = s.local_buttons != 0;
        let Some(screen) = s.screen else { return false };
        if Instant::now() < s.cooldown {
            return false;
        }
        let Some(c) = s.source.as_mut() else {
            return false;
        };
        if c.last_seen.elapsed() > LEASE_TIMEOUT && c.lease.is_some() {
            drop(s);
            self.stop(Some("Connection lost; control returned locally".into()));
            return false;
        }
        if !c.entered {
            if let Captured::Move { x, y, dx, .. } = event {
                let crossing = match c.edge {
                    Edge::Right => x >= screen.x + screen.width - 2.0 && dx > 0.0,
                    Edge::Left => x <= screen.x + 1.0 && dx < 0.0,
                };
                if crossing && !holding_local_button && c.remote.is_some() && c.lease.is_none() {
                    let lease = Uuid::new_v4();
                    let y = ((y - screen.y) / (screen.height - 1.0)).clamp(0.0, 1.0);
                    if let Some(remote) = c.remote {
                        c.point = remote.entry(c.edge, y);
                    }
                    c.lease = Some(lease);
                    c.requested = Instant::now();
                    c.last_seen = Instant::now();
                    if self
                        .tx
                        .try_send(Outbound {
                            peer: c.peer,
                            wire: Wire::Enter {
                                lease,
                                edge: c.edge,
                                y,
                            },
                        })
                        .is_err()
                    {
                        c.lease = None;
                    }
                }
            }
            return false;
        }
        let Some(remote) = c.remote else { return false };
        let lease = c.lease.unwrap();
        let mut returning = false;
        let mouse = match event {
            Captured::Move { dx, dy, .. } => {
                c.point.0 += dx;
                c.point.1 = (c.point.1 + dy).clamp(remote.y, remote.y + remote.height - 1.0);
                returning = match c.edge {
                    Edge::Right => c.point.0 < remote.x,
                    Edge::Left => c.point.0 >= remote.x + remote.width,
                };
                c.point.0 = c.point.0.clamp(remote.x, remote.x + remote.width - 1.0);
                MouseEvent::Move {
                    x: c.point.0,
                    y: c.point.1,
                }
            }
            Captured::Down(button) => MouseEvent::ButtonDown {
                button,
                x: c.point.0,
                y: c.point.1,
            },
            Captured::Up(button) => MouseEvent::ButtonUp {
                button,
                x: c.point.0,
                y: c.point.1,
            },
            Captured::Scroll(delta_x, delta_y) => MouseEvent::Scroll { delta_x, delta_y },
            Captured::Escape => unreachable!(),
        };
        let peer = c.peer;
        let wire = if returning {
            Wire::Release {
                lease,
                y: (c.point.1 - remote.y) / (remote.height - 1.0),
            }
        } else {
            Wire::Mouse {
                lease,
                event: mouse,
            }
        };
        if self.tx.try_send(Outbound { peer, wire }).is_err() {
            drop(s);
            self.stop(Some("Input queue full; control returned locally".into()));
            return false;
        }
        if returning {
            c.entered = false;
            c.lease = None;
            let y =
                screen.y + (c.point.1 - remote.y) / (remote.height - 1.0) * (screen.height - 1.0);
            let x = match c.edge {
                Edge::Right => screen.x + screen.width - 12.0,
                Edge::Left => screen.x + 11.0,
            };
            s.warp = Some((x, y));
            s.cooldown = Instant::now() + Duration::from_millis(350);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn armed() -> (CursorController, mpsc::Receiver<Outbound>, Uuid) {
        let (c, rx) = CursorController::new();
        let peer = Uuid::new_v4();
        {
            let mut s = c.state.lock().unwrap();
            s.capture_running = true;
            s.screen = Some(Screen {
                x: 0.,
                y: 0.,
                width: 1000.,
                height: 800.,
            });
        }
        c.arm(peer, Edge::Right).unwrap();
        {
            let mut s = c.state.lock().unwrap();
            s.source.as_mut().unwrap().remote = Some(Screen {
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            });
        }
        (c, rx, peer)
    }
    #[test]
    fn never_suppresses_before_receiver_acknowledges() {
        let (c, _rx, _) = armed();
        assert!(!c.capture(Captured::Move {
            x: 999.,
            y: 400.,
            dx: 2.,
            dy: 0.
        }));
        assert_eq!(c.status().phase, "entering");
    }
    #[test]
    fn reverse_crossing_restores_local_control() {
        let (c, _rx, _) = armed();
        c.capture(Captured::Move {
            x: 999.,
            y: 400.,
            dx: 2.,
            dy: 0.,
        });
        {
            c.state.lock().unwrap().source.as_mut().unwrap().entered = true;
        }
        assert!(c.capture(Captured::Move {
            x: 999.,
            y: 400.,
            dx: -10.,
            dy: 0.
        }));
        assert_eq!(c.status().phase, "armed");
        assert!(c.state.lock().unwrap().warp.is_some());
    }
    #[test]
    fn escape_and_expired_lease_fail_open_locally() {
        let (c, _, _) = armed();
        c.capture(Captured::Escape);
        assert_eq!(c.status().phase, "local");
        let (c, _, _) = armed();
        {
            let mut s = c.state.lock().unwrap();
            let c = s.source.as_mut().unwrap();
            c.lease = Some(Uuid::new_v4());
            c.entered = true;
            c.last_seen = Instant::now() - Duration::from_secs(4);
        }
        assert!(!c.capture(Captured::Down(0)));
        assert_eq!(c.status().phase, "local");
    }
    #[test]
    fn screen_validation_rejects_invalid_geometry() {
        assert!(
            !Screen {
                x: 0.,
                y: 0.,
                width: f64::NAN,
                height: 900.
            }
            .valid()
        );
        assert!(
            !Screen {
                x: 0.,
                y: 0.,
                width: 0.,
                height: 900.
            }
            .valid()
        );
    }
    #[test]
    fn a_local_drag_never_crosses_without_a_release() {
        let (c, _rx, _) = armed();
        c.capture(Captured::Down(0));
        c.capture(Captured::Move {
            x: 999.,
            y: 400.,
            dx: 2.,
            dy: 0.,
        });
        assert_eq!(c.status().phase, "armed");
        c.capture(Captured::Up(0));
        c.capture(Captured::Move {
            x: 999.,
            y: 400.,
            dx: 2.,
            dy: 0.,
        });
        assert_eq!(c.status().phase, "entering");
    }
}
