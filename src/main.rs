mod config;
mod discovery;
mod ipc;
mod launcher;
mod trace;
mod ui;
mod win;
use config::HASH;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    ptr,
};
use win::*;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Owned(Handle);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
unsafe fn modules(pid: u32) -> Result<HashMap<String, (usize, u32)>> {
    let s = CreateToolhelp32Snapshot(0x18, pid);
    if s as isize == -1 {
        return Err(std::io::Error::last_os_error().into());
    }
    let s = Owned(s);
    let mut m: Module = std::mem::zeroed();
    m.size = std::mem::size_of::<Module>() as u32;
    let mut ok = Module32FirstW(s.0, &mut m);
    let mut out = HashMap::new();
    while ok != 0 {
        out.insert(text(&m.name).to_lowercase(), (m.base, m.bytes));
        ok = Module32NextW(s.0, &mut m);
    }
    Ok(out)
}
fn root() -> Result<PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .ok_or("Missing exe directory")?
        .to_path_buf())
}
fn verify() -> Result<String> {
    verify_game(&discovery::locate()?)
}
fn verify_game(game: &Path) -> Result<String> {
    let assembly = game.join(r"Frosthaven_Data\Managed\Assembly-CSharp.dll");
    let hash = format!("{:x}", Sha256::digest(fs::read(&assembly)?));
    if hash != HASH {
        return Err(format!(
            "Unsupported game assembly SHA256\nAssembly: {}\nExpected: {HASH}\nActual: {hash}",
            assembly.display()
        )
        .into());
    }
    Ok(hash)
}
unsafe fn inject(pid: u32, diagnostics: &trace::Diagnostics) -> Result<()> {
    diagnostics.record(format!(
        "Injecting into PID {pid}: validating installation and assembly hash"
    ));
    let game = discovery::for_process(pid)?;
    verify_game(&game)?;
    diagnostics.record(format!("OpenProcess PID={pid}, access=0x43a"));
    let h = OpenProcess(0x43a, 0, pid);
    if h.is_null() {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(5) {
            return Err(
                trace::context("Access to the game process was denied. Close the game and reopen this tool as administrator", error),
            );
        }
        return Err(trace::context("OpenProcess failed", error));
    }
    let h = Owned(h);
    let mut name = vec![0u16; 32768];
    let mut len = name.len() as u32;
    if QueryFullProcessImageNameW(h.0, 0, name.as_mut_ptr(), &mut len) == 0 {
        return Err(trace::context(
            "QueryFullProcessImageNameW failed",
            std::io::Error::last_os_error(),
        ));
    }
    if !discovery::same_path(Path::new(&text(&name)), &game.join("Frosthaven.exe")) {
        return Err("Refusing non-target process".into());
    }
    let remote = modules(pid)?;
    if !remote.contains_key("mono-2.0-bdwgc.dll") {
        return Err("Mono not ready".into());
    }
    if remote.contains_key("lanbridge.dll") {
        return Err("Already injected. Close the game manually, then reopen this tool.".into());
    }
    let load = GetProcAddress(
        GetModuleHandleA(b"kernel32.dll\0".as_ptr()),
        b"LoadLibraryW\0".as_ptr(),
    ) as usize;
    if load == 0 {
        return Err(trace::context(
            "GetProcAddress(LoadLibraryW) failed",
            std::io::Error::last_os_error(),
        ));
    }
    let local = modules(std::process::id())?;
    let (owner, (base, _)) = local
        .iter()
        .find(|(_, (b, s))| load >= *b && load < *b + *s as usize)
        .ok_or("Cannot resolve LoadLibraryW module")?;
    let address = remote.get(owner).ok_or("Remote module missing")?.0 + load - base;
    let dll = wide(
        root()?
            .join("LanBridge.dll")
            .to_str()
            .ok_or("Non Unicode path")?,
    );
    let size = dll.len() * 2;
    diagnostics.record(format!(
        "LoadLibraryW remote address=0x{address:x}; bridge={}; UTF-16 buffer={size} bytes",
        root()?.join("LanBridge.dll").display()
    ));
    let mem = VirtualAllocEx(h.0, ptr::null_mut(), size, 0x3000, 4);
    if mem.is_null() {
        return Err(trace::context(
            "VirtualAllocEx failed",
            std::io::Error::last_os_error(),
        ));
    }
    diagnostics.record("Remote buffer allocated; writing DLL path");
    if WriteProcessMemory(h.0, mem, dll.as_ptr().cast(), size, ptr::null_mut()) == 0 {
        let error = std::io::Error::last_os_error();
        VirtualFreeEx(h.0, mem, 0, 0x8000);
        return Err(trace::context("WriteProcessMemory failed", error));
    }
    diagnostics.record("DLL path written; creating remote LoadLibraryW thread");
    let t = CreateRemoteThread(
        h.0,
        ptr::null_mut(),
        0,
        std::mem::transmute(address),
        mem,
        0,
        ptr::null_mut(),
    );
    if t.is_null() {
        let error = std::io::Error::last_os_error();
        VirtualFreeEx(h.0, mem, 0, 0x8000);
        return Err(trace::context("CreateRemoteThread failed", error));
    }
    let t = Owned(t);
    let wait = WaitForSingleObject(t.0, 15000);
    let wait_error = (wait == u32::MAX).then(std::io::Error::last_os_error);
    diagnostics.record(format!("Remote thread wait result=0x{wait:08x}"));
    if let Some(error) = wait_error {
        return Err(trace::context(
            "WaitForSingleObject failed; remote buffer retained",
            error,
        ));
    }
    if wait != 0 {
        return Err(format!(
            "LoadLibrary timeout; remote buffer retained; wait result=0x{wait:08x}"
        )
        .into());
    }
    VirtualFreeEx(h.0, mem, 0, 0x8000);
    if !modules(pid)?.contains_key("lanbridge.dll") {
        return Err("DLL did not load".into());
    }
    diagnostics.record("LanBridge.dll confirmed in target process module list");
    Ok(())
}
fn command(_dir: &Path, s: &str, diagnostics: &mut trace::Diagnostics) -> Result<()> {
    if !matches!(s, "status" | "steam-status" | "restore") {
        return Err(ui::text(
            "支持的命令：status、steam-status、restore",
            "Supported commands: status, steam-status, restore",
        )
        .into());
    }
    let pids = game_pids()?;
    let [pid] = pids.as_slice() else {
        return Err(ui::text(
            "未找到唯一的游戏进程，无法发送命令。",
            "A single running game is required to send a command.",
        )
        .into());
    };
    diagnostics.connect(*pid)?;
    let channel = ipc::Channel::open(*pid).map_err(|error| {
        trace::context(
            "Game components are not ready; opening the memory channel failed",
            error,
        )
    })?;
    let id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    channel.write(ipc::REQUEST, &format!("{id}|{s}"));
    let prefix = format!("{id}|");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        if let Some(response) = channel.read(ipc::RESPONSE) {
            if let Some(text) = response.strip_prefix(&prefix) {
                if let Some(error) = text.strip_prefix("ERROR ") {
                    return Err(error.to_string().into());
                }
                println!("{}", ui::diagnostic(text));
                return Ok(());
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(ui::text(
                "等待游戏命令响应超时。",
                "Timed out waiting for the game command response.",
            )
            .into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
fn main() {
    unsafe {
        SetConsoleOutputCP(65001);
    }
    let mut diagnostics = trace::Diagnostics::default();
    diagnostics.record(format!(
        "Launcher v{} PID={} locale={:?}",
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        ui::locale()
    ));
    let result = (|| -> Result<()> {
        let args: Vec<String> = std::env::args().collect();
        let dir = root()?;
        match args.get(1).map(String::as_str) {
            None => launcher::start(&dir, &mut diagnostics),
            Some("start") if args.len() == 2 => launcher::start(&dir, &mut diagnostics),
            Some("command") => command(&dir, &args[2..].join(" "), &mut diagnostics),
            Some("verify") => {
                println!("{}", verify()?);
                Ok(())
            }
            Some("locate") => { println!("{}", discovery::locate()?.display()); Ok(()) }
            _ => Err("Usage: frosthaven-lan [start | command <status|steam-status|restore> | locate | verify]".into()),
        }
    })();
    let failed = result.is_err();
    if failed {
        diagnostics.stop();
    }
    if let Err(e) = &result {
        eprintln!(
            "{}{}",
            ui::text("错误：", "Error: "),
            ui::diagnostic(&e.to_string())
        );
    }
    if let Err(error) = &result {
        eprintln!(
            "\n{}\n{}",
            ui::text(
                "完整错误与诊断日志（未截断）：",
                "Full error and diagnostic trace (untruncated):"
            ),
            diagnostics.report(error.as_ref())
        );
    }
    if failed {
        ui::pause_after_failure();
        std::process::exit(1);
    }
}
