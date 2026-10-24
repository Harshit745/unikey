//! Native mouse capture. Keyboard contents are never read or forwarded;
//! Escape is observed solely as the local emergency release.
use crate::cursor::{CursorController, Screen};
use anyhow::{Result, bail};

pub fn start(controller: CursorController) -> Result<()> {
    if controller.state.lock().unwrap().capture_running {
        return Ok(());
    }
    native::start(controller)
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use crate::cursor::Captured;
    use std::ffi::c_void;
    use std::ptr;
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Size {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    struct Rect {
        origin: Point,
        size: Size,
    }
    type Event = *mut c_void;
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn CGMainDisplayID() -> u32;
        fn CGDisplayBounds(display: u32) -> Rect;
        fn CGEventTapCreate(
            location: u32,
            place: u32,
            options: u32,
            mask: u64,
            callback: unsafe extern "C" fn(*mut c_void, u32, Event, *mut c_void) -> Event,
            data: *mut c_void,
        ) -> *mut c_void;
        fn CGEventTapEnable(tap: *mut c_void, enabled: bool);
        fn CGEventTapIsEnabled(tap: *mut c_void) -> bool;
        fn CGEventGetLocation(event: Event) -> Point;
        fn CGEventGetIntegerValueField(event: Event, field: u32) -> i64;
        fn CGEventGetDoubleValueField(event: Event, field: u32) -> f64;
        fn CGWarpMouseCursorPosition(point: Point) -> i32;
        fn CGDisplayHideCursor(display: u32) -> i32;
        fn CGDisplayShowCursor(display: u32) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopDefaultMode: *const c_void;
        fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            port: *mut c_void,
            order: isize,
        ) -> *mut c_void;
        fn CFRunLoopGetCurrent() -> *mut c_void;
        fn CFRunLoopAddSource(runloop: *mut c_void, source: *mut c_void, mode: *const c_void);
        fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after: bool) -> i32;
        fn CFRunLoopRemoveSource(runloop: *mut c_void, source: *mut c_void, mode: *const c_void);
        fn CFRelease(object: *const c_void);
    }
    pub fn screen() -> Result<Screen> {
        // CGDisplayBounds is in desktop coordinates, not Retina backing pixels.
        let r = unsafe { CGDisplayBounds(CGMainDisplayID()) };
        let s = Screen {
            x: r.origin.x,
            y: r.origin.y,
            width: r.size.width,
            height: r.size.height,
        };
        if !s.valid() {
            bail!("No usable main display");
        }
        Ok(s)
    }
    unsafe extern "C" fn callback(
        _: *mut c_void,
        kind: u32,
        event: Event,
        data: *mut c_void,
    ) -> Event {
        if event.is_null() || kind >= 0xffff_fffe {
            return event;
        }
        // SAFETY: the dedicated capture thread owns this boxed controller until
        // after the tap is disabled and its runloop source is removed.
        let controller = unsafe { &*(data as *const CursorController) };
        // Avoid recapturing software-injected events (including UniKey itself).
        if unsafe { CGEventGetIntegerValueField(event, 41) } != 0 {
            return event;
        }
        let captured = match kind {
            5 | 6 | 7 | 27 => {
                let p = unsafe { CGEventGetLocation(event) };
                Captured::Move {
                    x: p.x,
                    y: p.y,
                    dx: unsafe { CGEventGetDoubleValueField(event, 4) },
                    dy: unsafe { CGEventGetDoubleValueField(event, 5) },
                }
            }
            1 => Captured::Down(0),
            2 => Captured::Up(0),
            3 => Captured::Down(1),
            4 => Captured::Up(1),
            25 => Captured::Down(2),
            26 => Captured::Up(2),
            22 => Captured::Scroll(-unsafe { CGEventGetDoubleValueField(event, 12) }, -unsafe {
                CGEventGetDoubleValueField(event, 11)
            }),
            10 if unsafe { CGEventGetIntegerValueField(event, 9) } == 53 => Captured::Escape,
            _ => return event,
        };
        if controller.capture(captured) {
            ptr::null_mut()
        } else {
            event
        }
    }
    pub fn start(controller: CursorController) -> Result<()> {
        if !unsafe { AXIsProcessTrusted() } {
            bail!(
                "Allow UniKey in System Settings → Privacy & Security → Accessibility (Device Control and Data Access), then retry. Keyboard capture is not required."
            );
        }
        let display = screen()?;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("unikey-cursor".into())
            .spawn(move || {
                let mut controller = Box::new(controller);
                let mask = [1u32, 2, 3, 4, 5, 6, 7, 10, 22, 25, 26, 27]
                    .iter()
                    .fold(0u64, |mask, k| mask | (1u64 << k));
                // A filtering session tap suppresses local events only while the
                // authenticated receiver owns the cursor. The callback never blocks.
                let tap = unsafe {
                    CGEventTapCreate(
                        1,
                        0,
                        0,
                        mask,
                        callback,
                        (&mut *controller as *mut CursorController).cast(),
                    )
                };
                if tap.is_null() {
                    let _ = tx.send(Err(
                        "macOS refused the mouse event tap; check Accessibility permission"
                            .to_string(),
                    ));
                    return;
                }
                let source = unsafe { CFMachPortCreateRunLoopSource(ptr::null(), tap, 0) };
                if source.is_null() {
                    unsafe { CFRelease(tap) };
                    let _ = tx.send(Err("Cannot create mouse event runloop".into()));
                    return;
                }
                unsafe {
                    CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopDefaultMode);
                    CGEventTapEnable(tap, true);
                }
                {
                    let mut s = controller.state.lock().unwrap();
                    s.capture_running = true;
                    s.screen = Some(display);
                }
                let _ = tx.send(Ok(()));
                let mut hidden = false;
                loop {
                    unsafe {
                        CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.01, true);
                    }
                    let (running, remote, warp) = {
                        let mut s = controller.state.lock().unwrap();
                        (
                            s.capture_running,
                            s.source.as_ref().is_some_and(|c| c.entered),
                            s.warp.take(),
                        )
                    };
                    if remote != hidden {
                        unsafe {
                            if remote {
                                CGDisplayHideCursor(0);
                            } else {
                                CGDisplayShowCursor(0);
                            }
                        }
                        hidden = remote;
                    }
                    if let Some((x, y)) = warp {
                        unsafe {
                            CGWarpMouseCursorPosition(Point { x, y });
                        }
                    }
                    if !running {
                        break;
                    }
                    if !unsafe { CGEventTapIsEnabled(tap) } {
                        controller.stop(Some("macOS disabled mouse capture; retry sharing".into()));
                        break;
                    }
                }
                if hidden {
                    unsafe {
                        CGDisplayShowCursor(0);
                    }
                }
                unsafe {
                    CGEventTapEnable(tap, false);
                    CFRunLoopRemoveSource(CFRunLoopGetCurrent(), source, kCFRunLoopDefaultMode);
                    CFRelease(source);
                    CFRelease(tap);
                }
                controller.state.lock().unwrap().capture_running = false;
            })?;
        rx.recv_timeout(std::time::Duration::from_secs(3))?
            .map_err(anyhow::Error::msg)
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use crate::cursor::Captured;
    use std::{cell::RefCell, ptr};
    use windows_sys::Win32::{
        Foundation::{LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE},
            WindowsAndMessaging::*,
        },
    };
    thread_local! {static CONTROLLER:RefCell<Option<CursorController>>=const{RefCell::new(None)};}
    pub fn screen() -> Result<Screen> {
        let s = Screen {
            x: 0.,
            y: 0.,
            width: unsafe { GetSystemMetrics(SM_CXSCREEN) } as f64,
            height: unsafe { GetSystemMetrics(SM_CYSCREEN) } as f64,
        };
        if !s.valid() {
            bail!("No usable main display");
        }
        Ok(s)
    }
    unsafe extern "system" fn hook(code: i32, msg: WPARAM, data: LPARAM) -> LRESULT {
        if code < 0 {
            return unsafe { CallNextHookEx(ptr::null_mut(), code, msg, data) };
        }
        // SAFETY: Windows supplies an MSLLHOOKSTRUCT for every WH_MOUSE_LL callback.
        let e = unsafe { &*(data as *const MSLLHOOKSTRUCT) };
        if e.flags & 1 != 0 {
            return unsafe { CallNextHookEx(ptr::null_mut(), code, msg, data) };
        }
        let mut actual = POINT { x: 0, y: 0 };
        unsafe {
            GetCursorPos(&mut actual);
        }
        let event = match msg as u32 {
            WM_MOUSEMOVE => Captured::Move {
                x: e.pt.x as f64,
                y: e.pt.y as f64,
                dx: (e.pt.x - actual.x) as f64,
                dy: (e.pt.y - actual.y) as f64,
            },
            WM_LBUTTONDOWN => Captured::Down(0),
            WM_LBUTTONUP => Captured::Up(0),
            WM_RBUTTONDOWN => Captured::Down(1),
            WM_RBUTTONUP => Captured::Up(1),
            WM_MBUTTONDOWN => Captured::Down(2),
            WM_MBUTTONUP => Captured::Up(2),
            WM_MOUSEWHEEL => Captured::Scroll(0., -((e.mouseData >> 16) as i16 as f64) / 120.),
            WM_MOUSEHWHEEL => Captured::Scroll(((e.mouseData >> 16) as i16 as f64) / 120., 0.),
            _ => return unsafe { CallNextHookEx(ptr::null_mut(), code, msg, data) },
        };
        let suppress = CONTROLLER.with(|c| c.borrow().as_ref().is_some_and(|c| c.capture(event)));
        if suppress {
            1
        } else {
            unsafe { CallNextHookEx(ptr::null_mut(), code, msg, data) }
        }
    }
    pub fn start(controller: CursorController) -> Result<()> {
        let display = screen()?;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("unikey-cursor".into())
            .spawn(move || {
                CONTROLLER.with(|c| *c.borrow_mut() = Some(controller.clone()));
                let hook = unsafe {
                    SetWindowsHookExW(WH_MOUSE_LL, Some(hook), GetModuleHandleW(ptr::null()), 0)
                };
                if hook.is_null() {
                    let _ = tx.send(Err(format!(
                        "Mouse hook failed: {}",
                        std::io::Error::last_os_error()
                    )));
                    return;
                }
                {
                    let mut s = controller.state.lock().unwrap();
                    s.screen = Some(display);
                    s.capture_running = true;
                }
                let _ = tx.send(Ok(()));
                let mut hidden = false;
                loop {
                    let mut msg = unsafe { std::mem::zeroed() };
                    while unsafe { PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
                        unsafe {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                    if unsafe { GetAsyncKeyState(VK_ESCAPE as i32) } < 0 {
                        controller.capture(Captured::Escape);
                    }
                    let (running, remote, warp) = {
                        let mut s = controller.state.lock().unwrap();
                        (
                            s.capture_running,
                            s.source.as_ref().is_some_and(|c| c.entered),
                            s.warp.take(),
                        )
                    };
                    if remote != hidden {
                        unsafe {
                            ShowCursor(!remote as i32);
                        }
                        hidden = remote;
                    }
                    if let Some((x, y)) = warp {
                        unsafe {
                            SetCursorPos(x as i32, y as i32);
                        }
                    }
                    if !running {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(4));
                }
                if hidden {
                    unsafe {
                        ShowCursor(1);
                    }
                }
                unsafe {
                    UnhookWindowsHookEx(hook);
                }
                CONTROLLER.with(|c| *c.borrow_mut() = None);
            })?;
        rx.recv_timeout(std::time::Duration::from_secs(3))?
            .map_err(anyhow::Error::msg)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod native {
    use super::*;
    pub fn screen() -> Result<Screen> {
        bail!("Shared cursor currently supports macOS and Windows only")
    }
    pub fn start(_: CursorController) -> Result<()> {
        bail!("Native cursor capture is unavailable on this platform")
    }
}
