#[cfg(windows)]
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Mutex;
#[cfg(windows)]
use std::thread::{self, JoinHandle};
#[cfg(windows)]
use std::time::Duration;

#[derive(Default)]
pub struct PowerManager {
    inner: Mutex<PowerState>,
}

#[derive(Default)]
struct PowerState {
    handle: usize,
    system: bool,
    display: bool,
    #[cfg(windows)]
    keepalive: Option<(Sender<()>, JoinHandle<()>)>,
}

impl PowerManager {
    pub fn apply(&self, system: bool, display: bool) -> Result<(), String> {
        let mut state = self.inner.lock().map_err(|error| error.to_string())?;
        // A display request also needs a system request to keep the display on.
        let system_request = system || display;
        #[cfg(windows)]
        {
            if state.handle == 0 && system_request {
                state.handle = windows::create_request()?;
            }
            if state.handle != 0 {
                windows::set_request(state.handle, 1, &mut state.system, system_request)?;
                windows::set_request(state.handle, 0, &mut state.display, display)?;
                if display && state.keepalive.is_none() {
                    // Power requests do not reset the user-input idle timer used by auto-lock.
                    let (stop, receiver) = mpsc::channel();
                    let worker = thread::Builder::new()
                        .name("identityv-session-keepalive".into())
                        .spawn(move || {
                            windows::refresh_user_idle();
                            while matches!(
                                receiver.recv_timeout(Duration::from_secs(15)),
                                Err(RecvTimeoutError::Timeout)
                            ) {
                                windows::refresh_user_idle();
                            }
                        })
                        .map_err(|error| error.to_string())?;
                    state.keepalive = Some((stop, worker));
                } else if !display {
                    if let Some((stop, worker)) = state.keepalive.take() {
                        let _ = stop.send(());
                        let _ = worker.join();
                    }
                }
                if !system && !display {
                    windows::close_request(state.handle);
                    state.handle = 0;
                }
            }
        }
        #[cfg(not(windows))]
        {
            state.system = system_request;
            state.display = display;
        }
        Ok(())
    }
}

impl Drop for PowerManager {
    fn drop(&mut self) {
        let _ = self.apply(false, false);
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::iter;

    #[repr(C)]
    struct ReasonContext {
        version: u32,
        flags: u32,
        simple_reason: *const u16,
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    struct MouseInput {
        dx: i32,
        dy: i32,
        mouse_data: u32,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    struct KeyboardInput {
        key: u16,
        scan: u16,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[repr(C)]
    union InputData {
        mouse: MouseInput,
        keyboard: KeyboardInput,
    }

    #[repr(C)]
    struct Input {
        kind: u32,
        data: InputData,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn PowerCreateRequest(context: *const ReasonContext) -> *mut c_void;
        fn PowerSetRequest(handle: *mut c_void, request_type: u32) -> i32;
        fn PowerClearRequest(handle: *mut c_void, request_type: u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn SendInput(count: u32, input: *const Input, size: i32) -> u32;
    }

    pub fn refresh_user_idle() {
        // F15 is reserved for apps but normally has no game action. The paired
        // press and release refreshes Windows' user-input idle timer.
        let keys = [
            Input {
                kind: 1,
                data: InputData {
                    keyboard: KeyboardInput {
                        key: 0x7e,
                        scan: 0,
                        flags: 0,
                        time: 0,
                        extra_info: 0,
                    },
                },
            },
            Input {
                kind: 1,
                data: InputData {
                    keyboard: KeyboardInput {
                        key: 0x7e,
                        scan: 0,
                        flags: 2,
                        time: 0,
                        extra_info: 0,
                    },
                },
            },
        ];
        unsafe { SendInput(2, keys.as_ptr(), std::mem::size_of::<Input>() as i32) };
    }

    pub fn create_request() -> Result<usize, String> {
        let reason: Vec<u16> = "Identity V Assistant automation"
            .encode_utf16()
            .chain(iter::once(0))
            .collect();
        let context = ReasonContext {
            version: 0,
            flags: 1,
            simple_reason: reason.as_ptr(),
        };
        let handle = unsafe { PowerCreateRequest(&context) };
        if handle as isize == -1 || handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(handle as usize)
    }

    pub fn set_request(
        handle: usize,
        request_type: u32,
        current: &mut bool,
        desired: bool,
    ) -> Result<(), String> {
        if *current == desired {
            return Ok(());
        }
        let success = unsafe {
            if desired {
                PowerSetRequest(handle as *mut c_void, request_type)
            } else {
                PowerClearRequest(handle as *mut c_void, request_type)
            }
        };
        if success == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        *current = desired;
        Ok(())
    }

    pub fn close_request(handle: usize) {
        unsafe { CloseHandle(handle as *mut c_void) };
    }
}
