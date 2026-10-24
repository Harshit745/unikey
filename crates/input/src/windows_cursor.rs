//! Physical-coordinate scope and source cursor hiding without changing the
//! user's system cursor scheme, attaching input queues, or confining input.
use crate::cursor::Screen;
use anyhow::{Context, Result, bail};
use std::ptr;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{BLACK_BRUSH, GetStockObject},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        HiDpi::{
            DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            SetThreadDpiAwarenessContext,
        },
        WindowsAndMessaging::*,
    },
};

pub(crate) struct PhysicalCoordinates(DPI_AWARENESS_CONTEXT);
impl PhysicalCoordinates {
    pub(crate) fn enter() -> Result<Self> {
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if previous.is_null() {
            return Err(std::io::Error::last_os_error())
                .context("cannot select physical cursor coordinates");
        }
        Ok(Self(previous))
    }
}
impl Drop for PhysicalCoordinates {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}

unsafe extern "system" fn cursor_window(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_SETCURSOR | WM_MOUSEMOVE => {
            unsafe {
                SetCursor(ptr::null_mut());
            }
            1
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        _ => unsafe { DefWindowProcW(hwnd, msg, w, l) },
    }
}

/// ShowCursor is per-input-thread: calling it on a background hook thread
/// cannot hide the cursor over another app. A tiny, non-activating parking
/// window owns the hidden source point while remote control is active. The
/// low-level hook still suppresses local clicks/motion. Destroying the process
/// also destroys this window; no global cursor setting needs crash recovery.
pub(crate) struct SourceCursor {
    hwnd: HWND,
    class: Vec<u16>,
    mode: u8,
}
impl SourceCursor {
    pub(crate) fn new() -> Result<Self> {
        let class: Vec<u16> = "UniKeyCursorParking"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let instance = unsafe { GetModuleHandleW(ptr::null()) };
        let mut wc: WNDCLASSW = unsafe { std::mem::zeroed() };
        wc.hInstance = instance;
        wc.lpszClassName = class.as_ptr();
        wc.lpfnWndProc = Some(cursor_window);
        wc.hbrBackground = unsafe { GetStockObject(BLACK_BRUSH) } as _;
        if unsafe { RegisterClassW(&wc) } == 0 {
            return Err(std::io::Error::last_os_error())
                .context("cannot register cursor parking window");
        }
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class.as_ptr(),
                class.as_ptr(),
                WS_POPUP,
                0,
                0,
                8,
                8,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null(),
            )
        };
        if hwnd.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe {
                UnregisterClassW(class.as_ptr(), instance);
            }
            return Err(error).context("cannot create cursor parking window");
        }
        let cursor = Self {
            hwnd,
            class,
            mode: 0,
        };
        if unsafe { SetLayeredWindowAttributes(hwnd, 0, 1, LWA_ALPHA) } == 0 {
            return Err(std::io::Error::last_os_error())
                .context("cannot make cursor parking window transparent");
        }
        Ok(cursor)
    }

    pub(crate) fn update(
        &mut self,
        source_remote: bool,
        receiver_idle: bool,
        screen: Screen,
    ) -> Result<()> {
        let mode = if source_remote {
            1
        } else if receiver_idle {
            2
        } else {
            0
        };
        if mode == self.mode {
            return Ok(());
        }
        if mode == 0 {
            unsafe {
                ShowWindow(self.hwnd, SW_HIDE);
                SetCursor(LoadCursorW(ptr::null_mut(), IDC_ARROW));
            }
            self.mode = 0;
            return Ok(());
        }
        let (x, y) = if source_remote {
            // Keep a frozen source away from physical boundaries so native
            // trackpad/mouse deltas can travel in both directions indefinitely.
            (
                (screen.x + screen.width / 2.).round() as i32,
                (screen.y + screen.height / 2.).round() as i32,
            )
        } else {
            let mut p = unsafe { std::mem::zeroed() };
            if unsafe { GetPhysicalCursorPos(&mut p) } == 0 {
                bail!("cannot read physical cursor position");
            }
            (p.x, p.y)
        };
        if unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x - 4,
                y - 4,
                8,
                8,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
        } == 0
        {
            bail!("cannot position cursor parking window");
        }
        self.mode = mode;
        if source_remote && unsafe { SetPhysicalCursorPos(x, y) } == 0 {
            self.update(false, false, screen)?;
            bail!("cannot park source cursor");
        }
        unsafe {
            SetCursor(ptr::null_mut());
        }
        Ok(())
    }
}
impl Drop for SourceCursor {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
            UnregisterClassW(self.class.as_ptr(), GetModuleHandleW(ptr::null()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::HiDpi::{
        AreDpiAwarenessContextsEqual, GetThreadDpiAwarenessContext,
    };
    #[test]
    fn physical_coordinate_scope_restores_callers_context() {
        let before = unsafe { GetThreadDpiAwarenessContext() };
        {
            let _scope = PhysicalCoordinates::enter().unwrap();
            assert_ne!(
                unsafe {
                    AreDpiAwarenessContextsEqual(
                        GetThreadDpiAwarenessContext(),
                        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                    )
                },
                0
            );
        }
        assert_ne!(
            unsafe { AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(), before) },
            0
        );
    }
    #[test]
    fn parking_window_is_hidden_and_destroyed_without_global_cursor_changes() {
        let cursor = SourceCursor::new().unwrap();
        let hwnd = cursor.hwnd;
        assert_eq!(unsafe { IsWindowVisible(hwnd) }, 0);
        drop(cursor);
        assert_eq!(unsafe { IsWindow(hwnd) }, 0);
    }
}
