mod win;
use std::{
    ffi::{c_void, CString},
    io::Write,
    path::PathBuf,
    ptr,
};
use win::*;
unsafe fn directory(h: Handle) -> PathBuf {
    let mut p = [0u16; 32768];
    GetModuleFileNameW(h, p.as_mut_ptr(), p.len() as u32);
    PathBuf::from(text(&p)).parent().unwrap().to_path_buf()
}
unsafe extern "system" fn worker(h: Handle) -> u32 {
    let dir = directory(h);
    let log = |s: &str| {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("native.log"))
        {
            let _ = writeln!(f, "{s}");
        }
    };
    let m = GetModuleHandleA(b"mono-2.0-bdwgc.dll\0".as_ptr());
    if m.is_null() {
        log("Mono missing");
        return 1;
    }
    macro_rules! load {
        ($name:ident,$ty:ty) => {
            let p = GetProcAddress(m, concat!(stringify!($name), "\0").as_ptr());
            if p.is_null() {
                log(concat!("Missing ", stringify!($name)));
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
    let d = mono_get_root_domain();
    if d.is_null() {
        return 3;
    }
    let t = mono_thread_attach(d);
    let p = CString::new(dir.join("Bootstrap.dll").to_string_lossy().as_bytes()).unwrap();
    let a = mono_domain_assembly_open(d, p.as_ptr());
    if a.is_null() {
        log("Bootstrap load failed");
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
    log(if success {
        "Rust bridge: Bootstrap invoked"
    } else {
        "Managed entry failed"
    });
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
