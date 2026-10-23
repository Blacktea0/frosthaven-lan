mod config;
mod discovery;
mod input;
mod launcher;
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
    let hash = format!(
        "{:x}",
        Sha256::digest(fs::read(
            game.join(r"Frosthaven_Data\Managed\Assembly-CSharp.dll")
        )?)
    );
    if hash != HASH {
        return Err("Unsupported game assembly SHA256".into());
    }
    Ok(hash)
}
unsafe fn inject(pid: u32) -> Result<()> {
    let game = discovery::for_process(pid)?;
    verify_game(&game)?;
    let h = OpenProcess(0x43a, 0, pid);
    if h.is_null() {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(5) {
            return Err(
                "Access to the game process was denied. Steam-launched games require administrator permission for injection. Close the game manually, then reopen this tool using Run as administrator."
                    .into(),
            );
        }
        return Err(error.into());
    }
    let h = Owned(h);
    let mut name = vec![0u16; 32768];
    let mut len = name.len() as u32;
    if QueryFullProcessImageNameW(h.0, 0, name.as_mut_ptr(), &mut len) == 0 {
        return Err(std::io::Error::last_os_error().into());
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
    let mem = VirtualAllocEx(h.0, ptr::null_mut(), size, 0x3000, 4);
    if mem.is_null() {
        return Err(std::io::Error::last_os_error().into());
    }
    if WriteProcessMemory(h.0, mem, dll.as_ptr().cast(), size, ptr::null_mut()) == 0 {
        VirtualFreeEx(h.0, mem, 0, 0x8000);
        return Err(std::io::Error::last_os_error().into());
    }
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
        VirtualFreeEx(h.0, mem, 0, 0x8000);
        return Err(std::io::Error::last_os_error().into());
    }
    let t = Owned(t);
    if WaitForSingleObject(t.0, 15000) != 0 {
        return Err("LoadLibrary timeout; remote buffer retained".into());
    }
    VirtualFreeEx(h.0, mem, 0, 0x8000);
    if !modules(pid)?.contains_key("lanbridge.dll") {
        return Err("DLL did not load".into());
    }
    println!("Rust injector: loaded into Frosthaven PID {pid}");
    Ok(())
}
fn log(dir: &Path) -> String {
    fs::read_to_string(dir.join("payload.log")).unwrap_or_default()
}
fn command(dir: &Path, s: &str) -> Result<()> {
    if !matches!(s, "status" | "steam-status" | "restore") {
        return Err("Supported commands: status, steam-status, restore".into());
    }
    let p = dir.join("command.rust.tmp");
    fs::write(&p, s)?;
    fs::rename(p, dir.join("command.txt"))?;
    Ok(())
}
fn main() {
    let double_click = std::env::args_os().len() == 1;
    let result = (|| -> Result<()> {
        let args: Vec<String> = std::env::args().collect();
        let dir = root()?;
        match args.get(1).map(String::as_str) {
            None => launcher::start(&dir),
            Some("start") if args.len() == 2 => launcher::start(&dir),
            Some("command") => command(&dir, &args[2..].join(" ")),
            Some("verify") => {
                println!("{}", verify()?);
                Ok(())
            }
            Some("locate") => { println!("{}", discovery::locate()?.display()); Ok(()) }
            _ => Err("Usage: frosthaven-lan [start | command <status|steam-status|restore> | locate | verify]".into()),
        }
    })();
    let failed = result.is_err();
    if let Err(e) = &result {
        eprintln!("{e}");
        if double_click {
            if let Ok(dir) = root() {
                let _ = fs::write(dir.join("launcher-error.log"), e.to_string());
            }
        }
    }
    if double_click {
        let mut pid = 0;
        if unsafe { GetConsoleProcessList(&mut pid, 1) } == 1 {
            if failed {
                println!("Press Enter to close this window.");
            } else {
                println!("Press Enter to close this window. The game will keep running.");
            }
            input::wait_for_enter();
        }
    }
    if failed {
        std::process::exit(1);
    }
}
