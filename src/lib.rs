mod ipc;
mod trace;
mod win;
use std::{
    ffi::{c_void, CString},
    path::PathBuf,
    ptr,
};
use win::*;
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
    let _ = trace::send(pid, "Mono API exports resolved; waiting for root domain");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let d = loop {
        let domain = mono_get_root_domain();
        if !domain.is_null() {
            break domain;
        }
        if std::time::Instant::now() >= deadline {
            fail("Mono root domain initialization timed out");
            return 3;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
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
