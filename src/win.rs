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
    fn GetUserDefaultLocaleName(name: *mut u16, count: i32) -> i32;
    pub fn SetConsoleOutputCP(code_page: u32) -> i32;
    pub fn GetCurrentProcessId() -> u32;
    pub fn CreateNamedPipeW(
        name: *const u16,
        access: u32,
        mode: u32,
        instances: u32,
        out_size: u32,
        in_size: u32,
        timeout: u32,
        attributes: Handle,
    ) -> Handle;
    pub fn ConnectNamedPipe(pipe: Handle, overlapped: Handle) -> i32;
    pub fn DisconnectNamedPipe(pipe: Handle) -> i32;
    pub fn FlushFileBuffers(file: Handle) -> i32;
    pub fn CancelSynchronousIo(thread: Handle) -> i32;
    pub fn LocalFree(memory: Handle) -> Handle;
    pub fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        attributes: Handle,
        disposition: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    pub fn WaitNamedPipeW(name: *const u16, timeout: u32) -> i32;
    pub fn ReadFile(
        file: Handle,
        buffer: *mut u8,
        count: u32,
        read: *mut u32,
        overlapped: Handle,
    ) -> i32;
    pub fn WriteFile(
        file: Handle,
        buffer: *const u8,
        count: u32,
        written: *mut u32,
        overlapped: Handle,
    ) -> i32;
    pub fn CreateFileMappingW(
        file: Handle,
        attributes: Handle,
        protect: u32,
        high: u32,
        low: u32,
        name: *const u16,
    ) -> Handle;
    pub fn OpenFileMappingW(access: u32, inherit: i32, name: *const u16) -> Handle;
    pub fn MapViewOfFile(mapping: Handle, access: u32, high: u32, low: u32, bytes: usize)
        -> Handle;
    pub fn UnmapViewOfFile(address: *const c_void) -> i32;
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
    fn OpenProcessToken(process: Handle, access: u32, token: *mut Handle) -> i32;
    fn ConvertSidToStringSidW(sid: Handle, text: *mut *mut u16) -> i32;
    pub fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        text: *const u16,
        revision: u32,
        descriptor: *mut Handle,
        size: *mut u32,
    ) -> i32;
    fn GetTokenInformation(
        token: Handle,
        class: u32,
        info: *mut c_void,
        size: u32,
        returned: *mut u32,
    ) -> i32;
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

/// Read the effective process token, including UAC's filtered administrator tokens.
pub fn is_elevated() -> std::io::Result<bool> {
    unsafe {
        let mut token = std::ptr::null_mut();
        // GetCurrentProcess's documented pseudo-handle; TOKEN_QUERY = 0x0008.
        if OpenProcessToken(-1isize as Handle, 0x0008, &mut token) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut elevated = 0u32;
        let mut returned = 0u32;
        // TokenElevation = 20; TOKEN_ELEVATION contains one DWORD.
        let ok = GetTokenInformation(
            token,
            20,
            (&mut elevated as *mut u32).cast(),
            4,
            &mut returned,
        );
        let error = std::io::Error::last_os_error();
        CloseHandle(token);
        if ok == 0 {
            return Err(error);
        }
        Ok(elevated != 0)
    }
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

#[derive(Debug)]
struct ProcessPathError {
    operation: String,
    source: std::io::Error,
}
impl std::fmt::Display for ProcessPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.operation, self.source)
    }
}
impl std::error::Error for ProcessPathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
pub fn process_path(pid: u32) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    unsafe {
        let handle = OpenProcess(0x1000, 0, pid);
        if handle.is_null() {
            return Err(Box::new(ProcessPathError {
                operation: format!(
                    "OpenProcess failed while querying process path; PID={pid}, access=0x1000"
                ),
                source: std::io::Error::last_os_error(),
            }));
        }
        let mut buf = vec![0u16; 32768];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        let error = std::io::Error::last_os_error();
        CloseHandle(handle);
        if ok == 0 {
            return Err(Box::new(ProcessPathError {
                operation: format!("QueryFullProcessImageNameW failed; PID={pid}, flags=0, buffer capacity={}, returned size={size}", buf.len()),
                source: error,
            }));
        }
        Ok(std::path::PathBuf::from(text(&buf)))
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

#[cfg(test)]
mod tests {
    #[test]
    fn current_process_token_can_be_queried() {
        super::is_elevated()
            .expect("The current process token must support TokenElevation queries");
    }
}

pub fn user_locale() -> Option<String> {
    let mut name = [0u16; 85];
    (unsafe { GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32) } > 0)
        .then(|| text(&name))
}

pub fn current_user_sid() -> std::io::Result<String> {
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(-1isize as Handle, 0x0008, &mut token) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut bytes = 0;
        GetTokenInformation(token, 1, std::ptr::null_mut(), 0, &mut bytes);
        let mut data = vec![0u8; bytes as usize];
        let ok = GetTokenInformation(token, 1, data.as_mut_ptr().cast(), bytes, &mut bytes);
        let error = std::io::Error::last_os_error();
        CloseHandle(token);
        if ok == 0 {
            return Err(error);
        }
        let sid = std::ptr::read_unaligned(data.as_ptr().cast::<Handle>());
        let mut string_sid = std::ptr::null_mut();
        if ConvertSidToStringSidW(sid, &mut string_sid) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut count = 0;
        while *string_sid.add(count) != 0 {
            count += 1;
        }
        let result = String::from_utf16_lossy(std::slice::from_raw_parts(string_sid, count));
        LocalFree(string_sid.cast());
        Ok(result)
    }
}
