use super::config::APP_ID;
use super::{
    command, discovery, inject, log, modules, verify_game,
    win::{game_pids, is_elevated, loaded_module_path, open_steam_game},
    Result,
};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[derive(Default, Debug, PartialEq)]
struct Options {
    pid: Option<u32>,
}

fn wait_for_game(elevated: bool) -> Result<u32> {
    // Recheck immediately before dispatching to avoid an unnecessary second launch.
    match game_pids()?.as_slice() {
        [pid] => return Ok(*pid),
        [] => {}
        _ => return Err("Multiple game instances: specify --pid PID".into()),
    }
    if !elevated {
        return launch_direct();
    }
    open_steam_game(APP_ID)?;
    println!("Startup requested through Steam. Waiting for the game process...");
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        match game_pids()?.as_slice() {
            [pid] => return Ok(*pid),
            [] => {}
            _ => return Err("Multiple game instances: specify --pid PID".into()),
        }
        if Instant::now() >= deadline {
            return Err("Steam startup timed out. Check Steam and reopen this tool.".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn launch_direct() -> Result<u32> {
    let game = discovery::locate()?;
    verify_game(&game)?;
    println!("Launching Frosthaven directly with standard permissions...");
    let mut child = std::process::Command::new(game.join("Frosthaven.exe"))
        .current_dir(&game)
        .env("SteamAppId", APP_ID)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if game_pids()?.contains(&child.id()) {
            return Ok(child.id());
        }
        if let Some(status) = child.try_wait()? {
            return Err(format!("Direct game startup exited: {status}").into());
        }
        if Instant::now() >= deadline {
            return Err("Direct game startup timed out".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn options(args: &[String]) -> Result<Options> {
    let mut result = Options::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pid" if result.pid.is_none() => {
                let pid = args
                    .next()
                    .ok_or("--pid needs a process ID")?
                    .parse::<u32>()?;
                if pid == 0 {
                    return Err("PID must be positive".into());
                }
                result.pid = Some(pid);
            }
            _ => return Err(format!("Unknown or repeated start argument: {arg}").into()),
        }
    }
    Ok(result)
}

pub fn start(dir: &Path, args: &[String]) -> Result<()> {
    let options = options(args)?;
    let elevated = is_elevated()?;
    println!("Startup modes: double-click to launch the game directly with standard permissions.");
    println!("To launch through Steam, right-click this tool and select Run as administrator.");
    println!("Steam-launched game processes require administrator permission for injection due to Windows access restrictions.");
    println!(
        "Current mode: {}. An existing game process will be reused.",
        if elevated {
            "administrator (Steam startup)"
        } else {
            "standard (direct startup)"
        }
    );
    for file in [
        "LanBridge.dll",
        "Bootstrap.dll",
        "LanPayload.dll",
        "0Harmony.dll",
    ] {
        if !dir.join(file).is_file() {
            return Err(format!("Missing {file}; run cargo xtask build").into());
        }
    }
    let pids = game_pids()?;
    let pid = match options.pid {
        Some(pid) if pids.contains(&pid) => pid,
        Some(_) => return Err("--pid must identify a running Frosthaven process".into()),
        None if pids.len() > 1 => return Err("Multiple game instances: specify --pid PID".into()),
        None => match pids.first() {
            Some(pid) => *pid,
            None => wait_for_game(elevated)?,
        },
    };
    let game = discovery::for_process(pid)?;
    verify_game(&game)?;
    println!("Game directory: {}", game.display());
    if let Some(bridge) = loaded_module_path(pid, "LanBridge.dll") {
        let runtime = bridge.parent().ok_or("Invalid loaded bridge path")?;
        let offset = log(runtime).len();
        command(runtime, "status")?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let data = log(runtime);
            let fresh = data.get(offset..).unwrap_or("");
            if fresh.contains("STATUS backend=FFSNet.Backend.EnetNetworkBackend") {
                println!("The LAN patch is already enabled.");
                return Ok(());
            }
            if fresh.contains("STATUS backend=") || Instant::now() >= deadline {
                return Err("The patch is loaded but the LAN backend is not ready. Restart the game and run this tool again.".into());
            }
            thread::sleep(Duration::from_millis(250));
        }
    }
    let offset = log(dir).len();
    let bootstrap_offset = std::fs::read_to_string(dir.join("bootstrap.log"))
        .unwrap_or_default()
        .len();
    let native_offset = std::fs::read_to_string(dir.join("native.log"))
        .unwrap_or_default()
        .len();
    println!("Game PID {pid}; waiting for network initialization...");
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if !game_pids()?.contains(&pid) {
            return Err("Target game exited".into());
        }
        let loaded = match unsafe { modules(pid) } {
            Ok(modules) => modules.contains_key("mono-2.0-bdwgc.dll"),
            Err(error) => {
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.raw_os_error() == Some(5))
                {
                    return Err("Access to the game process was denied. Steam-launched games require administrator permission for injection. Close this tool, right-click it, and select Run as administrator.".into());
                }
                // Toolhelp can temporarily report ERROR_BAD_LENGTH while the
                // game is loading modules; retry only that transient failure.
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.raw_os_error() == Some(24))
                {
                    false
                } else {
                    return Err(error);
                }
            }
        };
        if loaded {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Timed out waiting for the game Mono runtime to initialize.".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
    unsafe {
        inject(pid)?;
    }
    let deadline = Instant::now() + Duration::from_secs(150);
    loop {
        if !game_pids()?.contains(&pid) {
            return Err("Game exited after injection".into());
        }
        let bootstrap = std::fs::read_to_string(dir.join("bootstrap.log")).unwrap_or_default();
        let fresh_bootstrap = bootstrap.get(bootstrap_offset..).unwrap_or("");
        if !fresh_bootstrap.trim().is_empty() {
            return Err(format!("Bootstrap failed: {fresh_bootstrap}").into());
        }
        let text = log(dir);
        let fresh = text.get(offset..).unwrap_or("");
        for failure in ["INSTALL ERROR", "PUMP ERROR", "REFUSED:"] {
            if fresh.contains(failure) {
                return Err(format!("Payload failed: {fresh}").into());
            }
        }
        if fresh.contains("BACKEND FFSNet.Backend.EnetNetworkBackend") {
            break;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Payload readiness timed out. Payload: {fresh}; native bridge: {}. Inspect logs in {}",
                std::fs::read_to_string(dir.join("native.log")).unwrap_or_default().get(native_offset..).unwrap_or(""),
                dir.display()
            )
            .into());
        }
        thread::sleep(Duration::from_millis(250));
    }
    println!(
        "LAN backend ready. Log: {}",
        dir.join("payload.log").display()
    );
    Ok(())
}
