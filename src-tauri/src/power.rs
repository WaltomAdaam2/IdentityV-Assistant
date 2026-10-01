use std::sync::Mutex;

#[derive(Default)]
pub struct PowerManager {
    inner: Mutex<PowerState>,
}

#[derive(Default)]
struct PowerState {
    handle: usize,
    system: bool,
    display: bool,
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

    #[link(name = "kernel32")]
    extern "system" {
        fn PowerCreateRequest(context: *const ReasonContext) -> *mut c_void;
        fn PowerSetRequest(handle: *mut c_void, request_type: u32) -> i32;
        fn PowerClearRequest(handle: *mut c_void, request_type: u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
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
