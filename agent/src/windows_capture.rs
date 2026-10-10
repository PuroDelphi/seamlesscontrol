//! Physical Windows input capture for a paired Omarchy receiver.
//! Hook callbacks only enqueue bounded events; network and clipboard work stay
//! off the hook thread so Windows does not silently remove a slow hook.

use crate::edge_policy::{EdgeGate, EdgePolicy};
use crate::state::InputEvent;
use crate::topology::{Edge, Rect, edge_fraction};
use crate::windows_keymap::set1_to_evdev;
use std::cell::RefCell;
use std::io;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorPos, GetForegroundWindow, GetMessageW, GetSystemMetrics,
    GetWindowRect, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLMHF_INJECTED, MSG,
    MSLLHOOKSTRUCT, PostThreadMessageW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SetCursorPos, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_QUIT, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_XBUTTONDBLCLK, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

const RELEASE_MESSAGE: u32 = WM_APP + 73;

#[derive(Debug)]
pub enum CaptureEvent {
    Begin(u16),
    Input(InputEvent),
    Release,
}

fn screen_rect() -> io::Result<Rect> {
    Rect::new(
        unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
        unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
        unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
        unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
    )
    .ok_or_else(|| io::Error::other("Windows virtual screen geometry is invalid"))
}

fn foreground_fullscreen() -> bool {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return false;
    }
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return false;
    }
    let mut bounds = RECT::default();
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetWindowRect(window, &mut bounds) } == 0
        || unsafe { GetMonitorInfoW(monitor, &mut info) } == 0
    {
        return true;
    }
    bounds.left <= info.rcMonitor.left + 2
        && bounds.top <= info.rcMonitor.top + 2
        && bounds.right >= info.rcMonitor.right - 2
        && bounds.bottom >= info.rcMonitor.bottom - 2
}

struct HookState {
    edge: Edge,
    policy: EdgePolicy,
    gate: EdgeGate,
    screen: Rect,
    sender: SyncSender<CaptureEvent>,
    failed: Arc<AtomicBool>,
    active: bool,
    previous: POINT,
    anchor: POINT,
    rearm_at: Instant,
}

impl HookState {
    fn send(&mut self, event: CaptureEvent) -> bool {
        if self.sender.try_send(event).is_ok() {
            true
        } else {
            // Stop swallowing physical input if the network worker falls behind.
            self.failed.store(true, Ordering::Release);
            self.active = false;
            false
        }
    }

    fn at_edge(&self, point: POINT) -> bool {
        match self.edge {
            Edge::Left => point.x <= self.screen.x,
            Edge::Right => point.x >= self.screen.right() - 1,
            Edge::Top => point.y <= self.screen.y,
            Edge::Bottom => point.y >= self.screen.bottom() - 1,
        }
    }

    fn release(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        self.rearm_at = Instant::now() + Duration::from_millis(700);
        let mut point = self.previous;
        match self.edge {
            Edge::Left => point.x = self.screen.x + 96,
            Edge::Right => point.x = self.screen.right() - 97,
            Edge::Top => point.y = self.screen.y + 96,
            Edge::Bottom => point.y = self.screen.bottom() - 97,
        }
        point.x = point.x.clamp(self.screen.x, self.screen.right() - 1);
        point.y = point.y.clamp(self.screen.y, self.screen.bottom() - 1);
        unsafe { SetCursorPos(point.x, point.y) };
        self.previous = point;
    }

    fn mouse(&mut self, message: u32, event: &MSLLHOOKSTRUCT) -> bool {
        if event.flags & LLMHF_INJECTED != 0 {
            return false;
        }
        if !self.active {
            if message == WM_MOUSEMOVE {
                let crossed = !self.at_edge(self.previous) && self.at_edge(event.pt);
                self.previous = event.pt;
                if crossed && Instant::now() >= self.rearm_at {
                    if !self
                        .gate
                        .allow(self.policy, foreground_fullscreen(), Instant::now())
                    {
                        return false;
                    }
                    let fraction = edge_fraction(&[self.screen], self.edge, event.pt.x, event.pt.y)
                        .unwrap_or(u16::MAX / 2);
                    let center = POINT {
                        x: self.screen.x + self.screen.width / 2,
                        y: self.screen.y + self.screen.height / 2,
                    };
                    if self.send(CaptureEvent::Begin(fraction))
                        && unsafe { SetCursorPos(center.x, center.y) } != 0
                    {
                        self.anchor = center;
                        self.active = true;
                        return true;
                    }
                    self.failed.store(true, Ordering::Release);
                }
            }
            return false;
        }

        let input = match message {
            WM_MOUSEMOVE => {
                let dx = (event.pt.x - self.anchor.x).clamp(-2048, 2048);
                let dy = (event.pt.y - self.anchor.y).clamp(-2048, 2048);
                (dx != 0 || dy != 0).then_some(InputEvent::Motion {
                    dx_milli: dx * 1000,
                    dy_milli: dy * 1000,
                })
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => Some(InputEvent::ButtonDown(272)),
            WM_LBUTTONUP => Some(InputEvent::ButtonUp(272)),
            WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => Some(InputEvent::ButtonDown(273)),
            WM_RBUTTONUP => Some(InputEvent::ButtonUp(273)),
            WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => Some(InputEvent::ButtonDown(274)),
            WM_MBUTTONUP => Some(InputEvent::ButtonUp(274)),
            WM_XBUTTONDOWN | WM_XBUTTONDBLCLK => {
                Some(InputEvent::ButtonDown(if event.mouseData >> 16 == 1 {
                    275
                } else {
                    276
                }))
            }
            WM_XBUTTONUP => Some(InputEvent::ButtonUp(if event.mouseData >> 16 == 1 {
                275
            } else {
                276
            })),
            WM_MOUSEWHEEL => Some(InputEvent::Scroll {
                horizontal_milli: 0,
                vertical_milli: -((event.mouseData >> 16) as i16 as i32) * 125,
            }),
            WM_MOUSEHWHEEL => Some(InputEvent::Scroll {
                horizontal_milli: ((event.mouseData >> 16) as i16 as i32) * 125,
                vertical_milli: 0,
            }),
            _ => None,
        };
        if let Some(event) = input {
            return self.send(CaptureEvent::Input(event));
        }
        true
    }

    fn keyboard(&mut self, message: u32, event: &KBDLLHOOKSTRUCT) -> bool {
        if !self.active || event.flags & LLKHF_INJECTED != 0 {
            return false;
        }
        let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
        if event.vkCode == 0x1b {
            if pressed {
                self.release();
                self.send(CaptureEvent::Release);
            }
            return true;
        }
        let extended = event.flags & LLKHF_EXTENDED != 0;
        if let Some(code) = set1_to_evdev(event.scanCode, extended) {
            let input = if pressed {
                InputEvent::KeyDown(code)
            } else {
                InputEvent::KeyUp(code)
            };
            return self.send(CaptureEvent::Input(input));
        }
        // Unknown keys remain local. The receiver cannot safely synthesize them.
        false
    }
}

thread_local! {
    static STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let consumed = STATE.try_with(|state| {
            state.try_borrow_mut().ok().and_then(|mut state| {
                state.as_mut().map(|state| {
                    // SAFETY: Windows supplies MSLLHOOKSTRUCT for WH_MOUSE_LL.
                    state.mouse(wparam as u32, unsafe {
                        &*(lparam as *const MSLLHOOKSTRUCT)
                    })
                })
            })
        });
        if matches!(consumed, Ok(Some(true))) {
            return 1;
        }
    }
    unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let consumed = STATE.try_with(|state| {
            state.try_borrow_mut().ok().and_then(|mut state| {
                state.as_mut().map(|state| {
                    // SAFETY: Windows supplies KBDLLHOOKSTRUCT for WH_KEYBOARD_LL.
                    state.keyboard(wparam as u32, unsafe {
                        &*(lparam as *const KBDLLHOOKSTRUCT)
                    })
                })
            })
        });
        if matches!(consumed, Ok(Some(true))) {
            return 1;
        }
    }
    unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) }
}

pub struct CaptureHandle {
    thread_id: u32,
    failed: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl CaptureHandle {
    pub fn start(
        edge: Edge,
        policy: EdgePolicy,
        sender: SyncSender<CaptureEvent>,
    ) -> io::Result<Self> {
        let failed = Arc::new(AtomicBool::new(false));
        let thread_failed = Arc::clone(&failed);
        let (ready_tx, ready_rx) = sync_channel(1);
        let thread = thread::spawn(move || {
            let result = (|| -> io::Result<()> {
                let screen = screen_rect()?;
                let mut previous = POINT::default();
                if unsafe { GetCursorPos(&mut previous) } == 0 {
                    return Err(io::Error::last_os_error());
                }
                STATE.with(|state| {
                    *state.borrow_mut() = Some(HookState {
                        edge,
                        policy,
                        gate: EdgeGate::default(),
                        screen,
                        sender,
                        failed: Arc::clone(&thread_failed),
                        active: false,
                        previous,
                        anchor: previous,
                        rearm_at: Instant::now(),
                    });
                });
                let module = unsafe { GetModuleHandleW(ptr::null()) };
                let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0) };
                if mouse.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let keyboard =
                    unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0) };
                if keyboard.is_null() {
                    unsafe { UnhookWindowsHookEx(mouse) };
                    return Err(io::Error::last_os_error());
                }
                let _ = ready_tx.send(Ok(unsafe { GetCurrentThreadId() }));
                let mut message = MSG::default();
                loop {
                    let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
                    if result <= 0 {
                        break;
                    }
                    if message.message == RELEASE_MESSAGE {
                        STATE.with(|state| {
                            if let Some(state) = state.borrow_mut().as_mut() {
                                state.release();
                            }
                        });
                    }
                }
                STATE.with(|state| {
                    if let Some(state) = state.borrow_mut().as_mut() {
                        state.release();
                    }
                });
                unsafe { UnhookWindowsHookEx(keyboard) };
                unsafe { UnhookWindowsHookEx(mouse) };
                Ok(())
            })();
            if let Err(error) = result {
                thread_failed.store(true, Ordering::Release);
                let _ = ready_tx.send(Err(error));
            }
            STATE.with(|state| *state.borrow_mut() = None);
        });
        let thread_id = ready_rx
            .recv()
            .map_err(|_| io::Error::other("Windows capture thread stopped during startup"))??;
        Ok(Self {
            thread_id,
            failed,
            thread: Some(thread),
        })
    }

    pub fn release(&self) {
        unsafe { PostThreadMessageW(self.thread_id, RELEASE_MESSAGE, 0, 0) };
    }

    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
