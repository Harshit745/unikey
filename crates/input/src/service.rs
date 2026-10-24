use crate::{
    cursor::{
        CursorController, CursorStatus, Edge, LEASE_TIMEOUT, Outbound, OutboundReceiver, Receiver,
        Wire,
    },
    event::{InputAction, KeyEvent, MouseEvent},
    platform::InputInjector,
};

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
use anyhow::{Result, bail};
use async_trait::async_trait;
use protocol::{MessageType, Packet};
use services::{Phase, Service, ServiceContext};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
#[cfg(test)]
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InputService {
    injector: Arc<dyn InputInjector>,
    pub cursor: CursorController,
    outbound: Arc<Mutex<Option<OutboundReceiver>>>,
    context: Arc<Mutex<Option<ServiceContext>>>,
    pairing: Option<Arc<pairing::PairingService>>,
}
impl Default for InputService {
    fn default() -> Self {
        Self::new()
    }
}

impl InputService {
    pub fn new() -> Self {
        let (cursor, rx) = CursorController::new();
        Self {
            injector: Arc::from(crate::platform::create_input_injector()),
            cursor,
            outbound: Arc::new(Mutex::new(Some(rx))),
            context: Arc::new(Mutex::new(None)),
            pairing: None,
        }
    }
    pub fn with_pairing(pairing: Arc<pairing::PairingService>) -> Self {
        let mut s = Self::new();
        s.pairing = Some(pairing);
        s
    }
    async fn trusted(&self, peer: &Uuid) -> bool {
        match &self.pairing {
            Some(p) => p.is_current_session_trusted(peer).await,
            None => false,
        }
    }
    pub fn status(&self) -> CursorStatus {
        self.cursor.status()
    }
    pub async fn start_cursor(&self, peer: Uuid, edge: Edge) -> Result<()> {
        if !self.trusted(&peer).await {
            bail!("Pair and connect the target device before sharing the cursor");
        }
        self.stop_cursor();
        crate::capture::start(self.cursor.clone())?;
        self.cursor.arm(peer, edge)
    }
    pub fn set_receiving(&self, enabled: bool) -> Result<()> {
        if enabled {
            if !self.injector.available() {
                bail!("Native mouse injection is unavailable");
            }
            crate::capture::start(self.cursor.clone())?;
        }
        self.cursor.state.lock().unwrap().receiving = enabled;
        if !enabled {
            self.release_receiver();
        }
        Ok(())
    }
    pub fn stop_cursor(&self) {
        self.cursor.stop(None);
        self.release_receiver();
    }
    fn release_receiver(&self) {
        let r = self.cursor.state.lock().unwrap().receiver.take();
        if let Some(r) = r {
            for button in r.buttons {
                let _ = self
                    .injector
                    .inject(&InputAction::Mouse(MouseEvent::ButtonUp {
                        button,
                        x: r.point.0,
                        y: r.point.1,
                    }));
            }
            let _ = self.cursor.tx.try_send(Outbound {
                peer: r.peer,
                wire: Wire::Release {
                    lease: r.lease,
                    y: 0.5,
                },
            });
        }
    }
    async fn send(&self, peer: Uuid, wire: Wire) -> Result<()> {
        if !self.trusted(&peer).await {
            bail!("Cursor peer is no longer trusted");
        }
        let ctx = self
            .context
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Input service is stopped"))?;
        let tx = ctx
            .session_manager
            .get_session_sender(&peer)
            .await
            .ok_or_else(|| anyhow::anyhow!("Cursor peer disconnected"))?;
        let message_type = match wire {
            Wire::Mouse { .. } => MessageType::InputMouseEvent,
            Wire::Release { .. } => MessageType::InputRelease,
            _ => MessageType::InputGrab,
        };
        let packet = Packet {
            version: 1,
            message_type,
            payload: serde_json::to_vec(&wire)?,
        };
        tokio::time::timeout(Duration::from_millis(250), tx.send(packet)).await??;
        Ok(())
    }
    async fn tick(&self) {
        let (source, receiver) = {
            let s = self.cursor.state.lock().unwrap();
            (
                s.source.as_ref().map(|c| {
                    (
                        c.peer,
                        c.lease,
                        c.entered,
                        c.requested,
                        c.last_seen,
                        c.remote.is_none(),
                    )
                }),
                s.receiver.as_ref().map(|r| (r.peer, r.lease, r.last_seen)),
            )
        };
        if let Some((peer, lease, entered, requested, seen, awaiting)) = source {
            if !self.trusted(&peer).await
                || (lease.is_some() && seen.elapsed() > LEASE_TIMEOUT)
                || ((awaiting || (lease.is_some() && !entered))
                    && requested.elapsed() > LEASE_TIMEOUT)
            {
                self.cursor.stop(Some("Peer unavailable or cursor handshake timed out. Enable receiving on an updated peer, then retry.".into()));
            } else if entered
                && self
                    .send(
                        peer,
                        Wire::Heartbeat {
                            lease: lease.unwrap(),
                        },
                    )
                    .await
                    .is_err()
            {
                self.cursor
                    .stop(Some("Connection lost; control returned locally".into()));
            }
        }
        if let Some((peer, lease, seen)) = receiver {
            let enabled = self.cursor.state.lock().unwrap().receiving;
            if !enabled
                || seen.elapsed() > LEASE_TIMEOUT
                || !self.trusted(&peer).await
                || self.send(peer, Wire::Heartbeat { lease }).await.is_err()
            {
                self.release_receiver();
            }
        }
    }
    async fn control(&self, peer: Uuid, wire: Wire) -> Result<()> {
        if !self.trusted(&peer).await {
            bail!("Untrusted cursor peer");
        }
        match wire {
            Wire::Probe { request } => {
                let screen = {
                    let s = self.cursor.state.lock().unwrap();
                    if !s.receiving || s.source.is_some() || s.receiver.is_some() {
                        bail!("Cursor receiving is disabled or busy");
                    }
                    s.screen
                };
                self.send(
                    peer,
                    Wire::Ready {
                        request,
                        screen: screen.ok_or_else(|| anyhow::anyhow!("No native display"))?,
                    },
                )
                .await?;
            }
            Wire::Ready { request, screen } => {
                if !screen.valid() {
                    bail!("Peer supplied invalid screen geometry");
                }
                let mut s = self.cursor.state.lock().unwrap();
                if let Some(c) = s
                    .source
                    .as_mut()
                    .filter(|c| c.peer == peer && c.request == request)
                {
                    c.remote = Some(screen);
                    c.last_seen = Instant::now();
                }
            }
            Wire::Enter { lease, edge, y } => {
                if !y.is_finite() || !(0.0..=1.0).contains(&y) {
                    bail!("Invalid entry position");
                }
                {
                    let mut s = self.cursor.state.lock().unwrap();
                    if !s.receiving || s.source.is_some() || s.receiver.is_some() {
                        bail!("Cursor receiving is disabled or busy");
                    }
                    let screen = s
                        .screen
                        .ok_or_else(|| anyhow::anyhow!("No native display"))?;
                    let x = match edge {
                        Edge::Right => screen.x + 3.,
                        Edge::Left => screen.x + screen.width - 4.,
                    };
                    let y = screen.y + y * (screen.height - 1.);
                    self.injector
                        .inject(&InputAction::Mouse(MouseEvent::Move { x, y }))?;
                    s.receiver = Some(Receiver {
                        peer,
                        lease,
                        last_seen: Instant::now(),
                        point: (x, y),
                        buttons: vec![],
                    });
                }
                if let Err(e) = self.send(peer, Wire::Entered { lease }).await {
                    self.release_receiver();
                    return Err(e);
                }
            }
            Wire::Entered { lease } => {
                let mut s = self.cursor.state.lock().unwrap();
                if let Some(c) = s.source.as_mut().filter(|c| {
                    c.peer == peer
                        && c.lease == Some(lease)
                        && c.requested.elapsed() < LEASE_TIMEOUT
                }) {
                    c.entered = true;
                    c.last_seen = Instant::now();
                }
            }
            Wire::Mouse { lease, event } => {
                let mut s = self.cursor.state.lock().unwrap();
                if !s.receiving {
                    bail!("Cursor receiving is disabled");
                }
                let screen = s.screen.ok_or_else(|| anyhow::anyhow!("No display"))?;
                let r = s
                    .receiver
                    .as_mut()
                    .filter(|r| {
                        r.peer == peer && r.lease == lease && r.last_seen.elapsed() < LEASE_TIMEOUT
                    })
                    .ok_or_else(|| anyhow::anyhow!("No cursor ownership for this packet"))?;
                let point = match &event {
                    MouseEvent::Move { x, y }
                    | MouseEvent::ButtonDown { x, y, .. }
                    | MouseEvent::ButtonUp { x, y, .. } => Some((*x, *y)),
                    MouseEvent::Scroll { delta_x, delta_y } => {
                        if !delta_x.is_finite()
                            || !delta_y.is_finite()
                            || delta_x.abs() > 100.
                            || delta_y.abs() > 100.
                        {
                            bail!("Invalid scroll delta");
                        }
                        None
                    }
                };
                if let Some((x, y)) = point {
                    if !x.is_finite()
                        || !y.is_finite()
                        || x < screen.x
                        || x >= screen.x + screen.width
                        || y < screen.y
                        || y >= screen.y + screen.height
                    {
                        bail!("Mouse position outside negotiated display");
                    }
                    r.point = (x, y);
                }
                if let MouseEvent::ButtonDown { button, .. } | MouseEvent::ButtonUp { button, .. } =
                    event
                    && button > 2
                {
                    bail!("Unsupported mouse button");
                }
                self.injector.inject(&InputAction::Mouse(event.clone()))?;
                match event {
                    MouseEvent::ButtonDown { button, .. } => {
                        if !r.buttons.contains(&button) {
                            r.buttons.push(button);
                        }
                    }
                    MouseEvent::ButtonUp { button, .. } => r.buttons.retain(|b| *b != button),
                    _ => {}
                }
                r.last_seen = Instant::now();
            }
            Wire::Heartbeat { lease } => {
                let mut s = self.cursor.state.lock().unwrap();
                if let Some(c) = s
                    .source
                    .as_mut()
                    .filter(|c| c.peer == peer && c.lease == Some(lease))
                {
                    c.last_seen = Instant::now();
                }
                if let Some(r) = s
                    .receiver
                    .as_mut()
                    .filter(|r| r.peer == peer && r.lease == lease)
                {
                    r.last_seen = Instant::now();
                }
            }
            Wire::Release { lease, .. } => {
                let (source, receiver) = {
                    let s = self.cursor.state.lock().unwrap();
                    (
                        s.source
                            .as_ref()
                            .is_some_and(|c| c.peer == peer && c.lease == Some(lease)),
                        s.receiver
                            .as_ref()
                            .is_some_and(|r| r.peer == peer && r.lease == lease),
                    )
                };
                if source {
                    self.cursor.stop(None);
                }
                if receiver {
                    self.release_receiver();
                }
            }
        }
        Ok(())
    }
    pub async fn forward_keyboard(
        &self,
        peer: &Uuid,
        event: KeyEvent,
        sm: &transport::session::SessionManager,
    ) -> Result<()> {
        let tx = sm
            .get_session_sender(peer)
            .await
            .ok_or_else(|| anyhow::anyhow!("Peer disconnected"))?;
        tx.send(Packet {
            version: 1,
            message_type: MessageType::InputKeyEvent,
            payload: serde_json::to_vec(&InputAction::Keyboard(event))?,
        })
        .await?;
        Ok(())
    }
    pub async fn forward_mouse(
        &self,
        peer: &Uuid,
        event: MouseEvent,
        sm: &transport::session::SessionManager,
    ) -> Result<()> {
        let tx = sm
            .get_session_sender(peer)
            .await
            .ok_or_else(|| anyhow::anyhow!("Peer disconnected"))?;
        tx.send(Packet {
            version: 1,
            message_type: MessageType::InputMouseEvent,
            payload: serde_json::to_vec(&InputAction::Mouse(event))?,
        })
        .await?;
        Ok(())
    }
}
#[async_trait]
impl Service for InputService {
    fn name(&self) -> &'static str {
        "input"
    }
    fn phase(&self) -> Phase {
        Phase::Phase1
    }
    async fn start(&self, ctx: ServiceContext) -> Result<()> {
        *self.context.lock().unwrap() = Some(ctx.clone());
        let rx = self.outbound.lock().unwrap().take();
        if let Some(mut rx) = rx {
            let this = self.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_millis(200));
                loop {
                    tokio::select! {
                        _=ctx.shutdown.cancelled()=>break,
                        event=rx.recv()=>{if this.send(event.peer,event.wire).await.is_err(){this.cursor.stop(Some("Cursor connection lost".into()));this.release_receiver();}},
                        _=tick.tick()=>this.tick().await,
                    }
                }
                this.stop_cursor();
                this.cursor.state.lock().unwrap().capture_running = false;
            });
        }
        tracing::info!("[INPUT] Started; shared cursor disabled until explicitly enabled");
        Ok(())
    }
    async fn stop(&self) -> Result<()> {
        self.stop_cursor();
        self.cursor.state.lock().unwrap().capture_running = false;
        Ok(())
    }
    async fn handle_packet(&self, peer: &Uuid, msg: MessageType, payload: &[u8]) -> Result<()> {
        if !self.trusted(peer).await {
            bail!("Untrusted input peer");
        }
        if matches!(
            msg,
            MessageType::InputGrab | MessageType::InputRelease | MessageType::InputMouseEvent
        ) && let Ok(wire) = serde_json::from_slice::<Wire>(payload)
        {
            return self.control(*peer, wire).await;
        }
        if matches!(
            msg,
            MessageType::InputKeyEvent | MessageType::InputMouseEvent
        ) {
            if !self.status().receiving_enabled {
                bail!("Input receiving is disabled");
            }
            let action: InputAction = serde_json::from_slice(payload)?;
            self.injector.inject(&action)?;
        }
        Ok(())
    }
}
