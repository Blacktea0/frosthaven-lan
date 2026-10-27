mod ipc;
mod trace;
mod win;
use std::{
    ffi::{c_void, CString},
    path::PathBuf,
    ptr,
};
use win::*;

const MONO_STARTUP_TIMEOUT: &str = "Mono root domain and main thread initialization timed out";

fn wait_for_runtime(
    timeout: std::time::Duration,
    mut probe: impl FnMut() -> (Handle, Handle),
    mut pause: impl FnMut(),
) -> Result<Handle, &'static str> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let (domain, main_thread) = probe();
        // The root domain is published before Mono initializes the thread class
        // and its vtable. Attaching at that point can abort the whole game.
        // Unity publishes the managed main thread after initializing Mono.
        // Only observe the pointers here; do not dereference managed objects on
        // this unattached native thread.
        if !domain.is_null() && !main_thread.is_null() {
            return Ok(domain);
        }
        if std::time::Instant::now() >= deadline {
            return Err(MONO_STARTUP_TIMEOUT);
        }
        pause();
    }
}

unsafe fn directory(h: Handle) -> PathBuf {
    let mut p = [0u16; 32768];
    GetModuleFileNameW(h, p.as_mut_ptr(), p.len() as u32);
    PathBuf::from(text(&p)).parent().unwrap().to_path_buf()
}
unsafe fn managed_exception_text(module: Handle, exception: Handle) -> Option<String> {
    let object_to_string = GetProcAddress(module, b"mono_object_to_string\0".as_ptr());
    let string_to_utf8 = GetProcAddress(module, b"mono_string_to_utf8\0".as_ptr());
    let free = GetProcAddress(module, b"mono_free\0".as_ptr());
    if object_to_string.is_null() || string_to_utf8.is_null() || free.is_null() {
        return None;
    }
    let object_to_string: unsafe extern "C" fn(Handle, *mut Handle) -> Handle =
        std::mem::transmute(object_to_string);
    let string_to_utf8: unsafe extern "C" fn(Handle) -> *mut i8 =
        std::mem::transmute(string_to_utf8);
    let free: unsafe extern "C" fn(Handle) = std::mem::transmute(free);
    let mut nested = ptr::null_mut();
    let text = object_to_string(exception, &mut nested);
    if text.is_null() || !nested.is_null() {
        return None;
    }
    let bytes = string_to_utf8(text);
    if bytes.is_null() {
        return None;
    }
    let result = std::ffi::CStr::from_ptr(bytes)
        .to_string_lossy()
        .into_owned();
    free(bytes.cast());
    Some(result)
}
unsafe extern "system" fn worker(h: Handle) -> u32 {
    let dir = directory(h);
    let pid = GetCurrentProcessId();
    let _ = trace::send(
        pid,
        &format!(
            "Native bridge worker entered; runtime directory={}",
            dir.display()
        ),
    );
    // Keep the mapping alive for the game's lifetime, including native startup failures.
    let channel = match ipc::Channel::create(GetCurrentProcessId()) {
        Ok(channel) => Box::leak(Box::new(channel)),
        Err(error) => {
            let _ = trace::send(pid, &format!("CreateFileMapping failed: {error:?}"));
            return 1;
        }
    };
    channel.write(ipc::STARTUP, "PENDING");
    let fail = |s: &str| {
        let _ = trace::send(pid, s);
        channel.write(ipc::STARTUP, &format!("ERROR {s}"));
    };
    let m = GetModuleHandleA(b"mono-2.0-bdwgc.dll\0".as_ptr());
    if m.is_null() {
        fail("Mono missing");
        return 1;
    }
    macro_rules! load {
        ($name:ident,$ty:ty) => {
            let p = GetProcAddress(m, concat!(stringify!($name), "\0").as_ptr());
            if p.is_null() {
                let error = std::io::Error::last_os_error();
                fail(&format!(
                    "Missing Mono export {}: {error:?}",
                    stringify!($name)
                ));
                return 2;
            }
            let $name: $ty = std::mem::transmute(p);
        };
    }
    load!(mono_get_root_domain, unsafe extern "C" fn() -> Handle);
    load!(mono_thread_get_main, unsafe extern "C" fn() -> Handle);
    load!(mono_thread_attach, unsafe extern "C" fn(Handle) -> Handle);
    load!(mono_thread_detach, unsafe extern "C" fn(Handle));
    load!(
        mono_domain_assembly_open,
        unsafe extern "C" fn(Handle, *const i8) -> Handle
    );
    load!(
        mono_assembly_get_image,
        unsafe extern "C" fn(Handle) -> Handle
    );
    load!(
        mono_class_from_name,
        unsafe extern "C" fn(Handle, *const u8, *const u8) -> Handle
    );
    load!(
        mono_class_get_method_from_name,
        unsafe extern "C" fn(Handle, *const u8, i32) -> Handle
    );
    load!(
        mono_runtime_invoke,
        unsafe extern "C" fn(Handle, Handle, *mut Handle, *mut Handle) -> Handle
    );
    let _ = trace::send(pid, "Mono API exports resolved; waiting for root domain and managed main thread; timeout=120 seconds");
    let mut last_state = None;
    let d = match wait_for_runtime(
        std::time::Duration::from_secs(120),
        || {
            let domain = mono_get_root_domain();
            let main_thread = mono_thread_get_main();
            let state = (domain as usize, main_thread as usize);
            if last_state != Some(state) {
                let _ = trace::send(pid, &format!("Mono startup state: root domain={domain:p}; managed main thread={main_thread:p}"));
                last_state = Some(state);
            }
            (domain, main_thread)
        },
        || std::thread::sleep(std::time::Duration::from_millis(50)),
    ) {
        Ok(domain) => domain,
        Err(error) => {
            fail(error);
            return 3;
        }
    };
    let _ = trace::send(
        pid,
        &format!(
            "Mono root domain and managed main thread detected; attaching worker; domain={d:p}"
        ),
    );
    let t = mono_thread_attach(d);
    if t.is_null() {
        fail("Mono thread attachment failed");
        return 3;
    }
    let _ = trace::send(pid, "Attached worker thread to Mono root domain");
    let p = CString::new(dir.join("Bootstrap.dll").to_string_lossy().as_bytes()).unwrap();
    let _ = trace::send(
        pid,
        &format!(
            "Loading managed bootstrap: {}",
            dir.join("Bootstrap.dll").display()
        ),
    );
    let a = mono_domain_assembly_open(d, p.as_ptr());
    if a.is_null() {
        fail("Bootstrap load failed");
        mono_thread_detach(t);
        return 4;
    }
    let c = mono_class_from_name(
        mono_assembly_get_image(a),
        b"FrosthavenLan\0".as_ptr(),
        b"Bootstrap\0".as_ptr(),
    );
    let method = if c.is_null() {
        ptr::null_mut()
    } else {
        mono_class_get_method_from_name(c, b"Start\0".as_ptr(), 0)
    };
    let mut ex: Handle = ptr::null_mut();
    if !method.is_null() {
        mono_runtime_invoke(method, ptr::null_mut(), ptr::null_mut(), &mut ex);
    }
    let success = !method.is_null() && ex.is_null();
    if success {
        let _ = trace::send(pid, "Bootstrap.Start invocation returned");
    }
    if !success {
        let details = if ex.is_null() {
            "Bootstrap.Start method was not found".to_string()
        } else {
            managed_exception_text(m, ex).unwrap_or_else(|| {
                format!("Managed exception at {ex:p}; Mono exception formatting unavailable")
            })
        };
        fail(&format!("Managed entry failed: {details}"));
    }
    mono_thread_detach(t);
    if success {
        0
    } else {
        5
    }
}
#[no_mangle]
pub unsafe extern "system" fn DllMain(h: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    if reason == 1 {
        DisableThreadLibraryCalls(h);
        let t = CreateThread(ptr::null_mut(), 0, worker, h, 0, ptr::null_mut());
        if !t.is_null() {
            CloseHandle(t);
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn root_domain_alone_does_not_allow_attachment() {
        // Simulate cold startup: the domain exists several polls before the
        // managed main thread. No game or Mono runtime is loaded by this test.
        let domain = 1usize as Handle;
        let main = 2usize as Handle;
        let mut states = [
            (ptr::null_mut(), ptr::null_mut()),
            (domain, ptr::null_mut()),
            (domain, ptr::null_mut()),
            (domain, main),
        ]
        .into_iter();
        let mut pauses = 0;
        let result = wait_for_runtime(
            Duration::from_secs(1),
            || states.next().unwrap(),
            || pauses += 1,
        );
        assert_eq!(result, Ok(domain));
        assert_eq!(pauses, 3);
    }

    #[test]
    fn incomplete_runtime_times_out_without_attaching() {
        for state in [
            (ptr::null_mut(), ptr::null_mut()),
            (1usize as Handle, ptr::null_mut()),
            (ptr::null_mut(), 2usize as Handle),
        ] {
            assert_eq!(
                wait_for_runtime(Duration::ZERO, || state, || panic!("must time out")),
                Err(MONO_STARTUP_TIMEOUT)
            );
        }
    }

    #[test]
    fn already_initialized_runtime_needs_no_delay() {
        let domain = 1usize as Handle;
        assert_eq!(
            wait_for_runtime(
                Duration::ZERO,
                || (domain, 2usize as Handle),
                || panic!("already ready")
            ),
            Ok(domain)
        );
    }
}
