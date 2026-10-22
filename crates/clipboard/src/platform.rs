use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A snapshot of clipboard content, transported as JSON between peers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipboardContent {
    /// MIME type of the payload: `text/plain`, `text/html` or `image/png`.
    pub content_type: String,
    /// Plain-text representation (always present for text, fallback for html).
    pub text: Option<String>,
    /// Rich-text HTML representation (when available).
    pub html: Option<String>,
    /// RGBA bitmap payload (when the clipboard holds an image).
    #[serde(default)]
    pub image: Option<ImagePayload>,
}

/// Raw RGBA image data carried over the wire as base64.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImagePayload {
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixel data, base64-encoded.
    pub rgba_base64: String,
}

/// Platform-specific clipboard read/write abstraction.
///
/// The concrete backends hold an `arboard::Clipboard` behind a mutex, which is
/// not `Sync` on every platform; the mutex restores `Send + Sync` so the
/// service can be shared across async tasks.
pub trait ClipboardAccess: Send + Sync + std::fmt::Debug {
    /// Read the current clipboard content (if any).
    fn read(&self) -> Result<Option<ClipboardContent>>;

    /// Write content to the system clipboard.
    fn write(&self, content: &ClipboardContent) -> Result<()>;

    /// Clone into a boxed trait object for ownership.
    fn clone_box(&self) -> Box<dyn ClipboardAccess>;
}

// ---------------------------------------------------------------------------
// arboard backend (macOS / Windows / Linux)
// ---------------------------------------------------------------------------

/// Real clipboard backend built on `arboard`. Supports text and images on
/// read; text, HTML and images on write.
///
/// Note: `arboard` removed HTML *reading* in v3.3, so rich-text content is
/// surfaced through its plain-text rendering on read and written back as HTML
/// only when a peer explicitly sends an HTML payload.
pub struct ArboardClipboard {
    inner: Mutex<arboard::Clipboard>,
}

impl std::fmt::Debug for ArboardClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArboardClipboard")
    }
}

impl ArboardClipboard {
    pub fn new() -> Result<Self> {
        let clipboard = arboard::Clipboard::new()
            .context("failed to initialize system clipboard")?;
        Ok(Self {
            inner: Mutex::new(clipboard),
        })
    }
}

impl Default for ArboardClipboard {
    fn default() -> Self {
        Self::new().expect("system clipboard must initialize")
    }
}

impl ClipboardAccess for ArboardClipboard {
    fn read(&self) -> Result<Option<ClipboardContent>> {
        let mut clip = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("clipboard mutex poisoned"))?;

        // 1. Plain text
        match clip.get_text() {
            Ok(text) => {
                return Ok(Some(ClipboardContent {
                    content_type: "text/plain".to_string(),
                    text: Some(text),
                    html: None,
                    image: None,
                }));
            }
            Err(arboard::Error::ContentNotAvailable) => {}
            Err(e) => return Err(e.into()),
        }

        // 2. Image
        match clip.get_image() {
            Ok(img) => {
                let bytes = img.bytes.into_owned();
                return Ok(Some(ClipboardContent {
                    content_type: "image/png".to_string(),
                    text: None,
                    html: None,
                    image: Some(ImagePayload {
                        width: img.width as u32,
                        height: img.height as u32,
                        rgba_base64: base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &bytes,
                        ),
                    }),
                }));
            }
            Err(arboard::Error::ContentNotAvailable) => {}
            Err(e) => return Err(e.into()),
        }

        Ok(None)
    }

    fn write(&self, content: &ClipboardContent) -> Result<()> {
        let mut clip = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("clipboard mutex poisoned"))?;

        if let Some(image) = &content.image {
            let bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &image.rgba_base64,
            )
            .context("invalid base64 image payload")?;
            let img = arboard::ImageData {
                width: image.width as usize,
                height: image.height as usize,
                bytes: std::borrow::Cow::Owned(bytes),
            };
            clip.set_image(img)?;
            return Ok(());
        }

        if let Some(html) = &content.html {
            let alt = content.text.clone().unwrap_or_default();
            clip.set_html(html.clone(), Some(alt))?;
            return Ok(());
        }

        if let Some(text) = &content.text {
            clip.set_text(text.clone())?;
            return Ok(());
        }

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ClipboardAccess> {
        Box::new(ArboardClipboard::new().unwrap_or_default())
    }
}

// ---------------------------------------------------------------------------
// Stub fallback (headless CI / unsupported environments)
// ---------------------------------------------------------------------------

/// No-op backend used when the real clipboard cannot be opened (headless CI).
#[derive(Debug, Clone)]
pub struct StubClipboard;

impl ClipboardAccess for StubClipboard {
    fn read(&self) -> Result<Option<ClipboardContent>> {
        Ok(None)
    }

    fn write(&self, _content: &ClipboardContent) -> Result<()> {
        tracing::debug!("[CLIPBOARD] Stub write (no system clipboard available)");
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ClipboardAccess> {
        Box::new(self.clone())
    }
}

/// Factory: return the real arboard backend, falling back to the stub when the
/// platform clipboard is unavailable (e.g. headless test runners).
pub fn create_clipboard_access() -> Box<dyn ClipboardAccess> {
    match ArboardClipboard::new() {
        Ok(clipboard) => Box::new(clipboard),
        Err(e) => {
            tracing::warn!("[CLIPBOARD] Falling back to stub backend: {e}");
            Box::new(StubClipboard)
        }
    }
}
