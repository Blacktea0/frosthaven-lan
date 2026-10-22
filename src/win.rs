#![allow(non_snake_case, dead_code)]
use std::ffi::c_void;
pub type Handle = *mut c_void;
#[repr(C)]
pub struct Module {
    pub size: u32,
    pub id: u32,
    pub pid: u32,
    pub global: u32,
    pub usage: u32,
    pub base: usize,
    pub bytes: u32,
    pub handle: Handle,
    pub name: [u16; 256],
    pub path: [u16; 260],
}
#[repr(C)]
pub struct Process {
    pub size: u32,
    pub usage: u32,
    pub pid: u32,
    pub heap: usize,
    pub module_id: u32,
    pub threads: u32,
    pub parent_pid: u32,
    pub priority: i32,
    pub flags: u32,
    pub exe: [u16; 260],
}
#[link(name = "kernel32")]
extern "system" {
    pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
    pub fn CloseHandle(h: Handle) -> i32;
    pub fn QueryFullProcessImageNameW(h: Handle, flags: u32, name: *mut u16, size: *mut u32)
        -> i32;
    pub fn VirtualAllocEx(h: Handle, p: Handle, size: usize, kind: u32, protect: u32) -> Handle;
    pub fn VirtualFreeEx(h: Handle, p: Handle, size: usize, kind: u32) -> i32;
    pub fn WriteProcessMemory(
        h: Handle,
        p: Handle,
        buf: *const c_void,
        size: usize,
        written: *mut usize,
    ) -> i32;
    pub fn CreateRemoteThread(
        h: Handle,
        attr: Handle,
        stack: usize,
        start: unsafe extern "system" fn(Handle) -> u32,
        arg: Handle,
        flags: u32,
        id: *mut u32,
    ) -> Handle;
    pub fn WaitForSingleObject(h: Handle, ms: u32) -> u32;
    pub fn GetConsoleProcessList(pids: *mut u32, count: u32) -> u32;
    pub fn GetModuleHandleA(name: *const u8) -> Handle;
    pub fn GetProcAddress(h: Handle, name: *const u8) -> *mut c_void;
    pub fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
    pub fn Module32FirstW(h: Handle, m: *mut Module) -> i32;
    pub fn Module32NextW(h: Handle, m: *mut Module) -> i32;
    pub fn Process32FirstW(h: Handle, p: *mut Process) -> i32;
    pub fn Process32NextW(h: Handle, p: *mut Process) -> i32;
    pub fn GetModuleFileNameW(h: Handle, path: *mut u16, size: u32) -> u32;
    pub fn DisableThreadLibraryCalls(h: Handle) -> i32;
    pub fn CreateThread(
        attr: Handle,
        stack: usize,
        start: unsafe extern "system" fn(Handle) -> u32,
        arg: Handle,
        flags: u32,
        id: *mut u32,
    ) -> Handle;
}
#[link(name = "advapi32")]
extern "system" {
    fn RegGetValueW(
        key: Handle,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut c_void,
        bytes: *mut u32,
    ) -> i32;
}
#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        window: Handle,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> Handle;
}

pub fn open_steam_game(app_id: &str) -> Result<(), String> {
    let uri = format!("steam://rungameid/{app_id}");
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide("open").as_ptr(),
            wide(&uri).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    } as isize;
    if result <= 32 {
        return Err(format!(
            "Cannot open {uri} (error {result}). Check that Steam is installed and the steam:// protocol is registered."
        ));
    }
    Ok(())
}

pub fn registry_string(machine: bool, subkey: &str, name: &str, view: u32) -> Option<String> {
    let key = if machine {
        0x80000002u32
    } else {
        0x80000001u32
    } as i32 as isize as Handle;
    let mut value = vec![0u16; 32768];
    let mut bytes = (value.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(
            key,
            wide(subkey).as_ptr(),
            wide(name).as_ptr(),
            2 | view,
            std::ptr::null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    (status == 0).then(|| text(&value))
}

pub fn process_path(pid: u32) -> std::io::Result<std::path::PathBuf> {
    unsafe {
        let handle = OpenProcess(0x1000, 0, pid);
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let mut buf = vec![0u16; 32768];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        let error = std::io::Error::last_os_error();
        CloseHandle(handle);
        if ok == 0 {
            return Err(error);
        }
        Ok(std::path::PathBuf::from(text(&buf)))
    }
}

pub fn loaded_module_path(pid: u32, name: &str) -> Option<std::path::PathBuf> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(0x18, pid);
        if snapshot as isize == -1 {
            return None;
        }
        let mut entry: Module = std::mem::zeroed();
        entry.size = std::mem::size_of::<Module>() as u32;
        let mut ok = Module32FirstW(snapshot, &mut entry);
        let mut found = None;
        while ok != 0 {
            if text(&entry.name).eq_ignore_ascii_case(name) {
                found = Some(std::path::PathBuf::from(text(&entry.path)));
                break;
            }
            ok = Module32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        found
    }
}
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn text(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|x| *x == 0).unwrap_or(s.len())])
}

pub fn game_pids() -> std::io::Result<Vec<u32>> {
    pids_named("Frosthaven.exe")
}

pub fn pids_named(name: &str) -> std::io::Result<Vec<u32>> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(2, 0);
        if snapshot as isize == -1 {
            return Err(std::io::Error::last_os_error());
        }
        let mut entry: Process = std::mem::zeroed();
        entry.size = std::mem::size_of::<Process>() as u32;
        let mut ok = Process32FirstW(snapshot, &mut entry);
        let mut pids = Vec::new();
        while ok != 0 {
            if text(&entry.exe).eq_ignore_ascii_case(name) {
                pids.push(entry.pid);
            }
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        Ok(pids)
    }
}
