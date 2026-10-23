use super::config::APP_ID;
use super::{
    discovery, inject, ipc, modules, trace, ui, verify_game,
    win::{game_pids, is_elevated, open_steam_game},
    Result,
};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

const GAME_ALREADY_RUNNING: &str = "Frosthaven is already running. Close all game instances manually, then reopen this tool. Existing game processes will not be injected.";

fn require_closed_game(pids: &[u32]) -> Result<()> {
    if !pids.is_empty() {
        return Err(GAME_ALREADY_RUNNING.into());
    }
    Ok(())
}

fn wait_for_game(elevated: bool, game: &Path) -> Result<u32> {
    // Recheck just before launching in case a game appeared after the initial check.
    require_closed_game(&game_pids()?)?;
    if !elevated {
        return launch_direct(game);
    }
    open_steam_game(APP_ID)?;
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        match game_pids()?.as_slice() {
            [pid] => return Ok(*pid),
            [] => {}
            _ => return Err("Multiple game instances detected. Close all game instances manually, then reopen this tool.".into()),
        }
        if Instant::now() >= deadline {
            return Err("Steam startup timed out. Check Steam and reopen this tool.".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn launch_direct(game: &Path) -> Result<u32> {
    require_closed_game(&game_pids()?)?;
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

pub fn start(dir: &Path, diagnostics: &mut trace::Diagnostics) -> Result<()> {
    diagnostics.record("Checking for existing game processes");
    require_closed_game(&game_pids()?)?;
    let elevated = is_elevated()?;
    diagnostics.record(format!(
        "Launch mode: elevated={elevated}; runtime directory={}",
        dir.display()
    ));
    let mode_progress = ui::Progress::new(if elevated {
        ui::text("管理员模式启动中", "Starting in administrator mode")
    } else {
        ui::text("非管理员模式启动中", "Starting in standard mode")
    });
    for file in [
        "LanBridge.dll",
        "Bootstrap.dll",
        "LanPayload.dll",
        "0Harmony.dll",
    ] {
        diagnostics.record(format!(
            "Runtime file {}: present={}",
            dir.join(file).display(),
            dir.join(file).is_file()
        ));
        if !dir.join(file).is_file() {
            return Err(format!("Missing {file}; run cargo xtask build").into());
        }
    }
    diagnostics.record("Locating Steam-registered Frosthaven installation");
    let game = discovery::locate()?;
    diagnostics.record(format!("Located game directory: {}", game.display()));
    let hash = verify_game(&game)?;
    diagnostics.record(format!("Game assembly SHA256 verified: {hash}"));
    drop(mode_progress);
    ui::line(format!(
        "{}{}",
        ui::text("定位到游戏目录：", "Game directory: "),
        game.display()
    ));
    let pid = {
        let _progress = ui::Progress::new(ui::text("正在启动游戏中", "Starting the game"));
        diagnostics.record(if elevated {
            "Requesting Steam startup"
        } else {
            "Starting game EXE directly"
        });
        wait_for_game(elevated, &game)?
    };
    diagnostics.record(format!("Detected game PID {pid}; validating process path"));
    diagnostics.connect(pid)?;
    let actual_game = discovery::for_process(pid)?;
    verify_game(&actual_game)?;
    let progress = ui::Progress::new(format!(
        "{}{}{}",
        ui::text("检测到游戏进程 (PID:", "Game process detected (PID: "),
        pid,
        ui::text(
            ")，正在等待网络组件初始化",
            "), waiting for network components to initialize"
        )
    ));
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
                    return Err(trace::context("Access to the game process was denied while reading modules; close the game and reopen this tool as administrator", error));
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
            diagnostics.record("Mono runtime module detected");
            break;
        }
        if Instant::now() >= deadline {
            return Err("Timed out waiting for the game Mono runtime to initialize.".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
    unsafe {
        inject(pid, diagnostics)?;
    }
    let deadline = Instant::now() + Duration::from_secs(150);
    let mut last_channel_error = None;
    let mut last_state = None;
    loop {
        if !game_pids()?.contains(&pid) {
            return Err("Game exited after injection".into());
        }
        match ipc::Channel::open(pid) {
            Ok(channel) => {
                last_state = channel.read(ipc::STARTUP);
                match last_state.as_deref() {
                    Some("READY") => {
                        diagnostics.record("Managed game thread confirmed LAN backend readiness");
                        break;
                    }
                    Some(message) if message.starts_with("ERROR ") => {
                        return Err(message[6..].to_string().into())
                    }
                    _ => {}
                }
            }
            Err(error) => {
                last_channel_error = Some(error);
            }
        }
        if Instant::now() >= deadline {
            diagnostics.record(format!("Last startup state: {last_state:?}; last shared-memory error: {last_channel_error:?}"));
            return Err(ui::text(
                "等待网络组件初始化超时，请关闭游戏后重新打开本工具。",
                "Network component initialization timed out. Close the game and reopen this tool.",
            )
            .into());
        }
        thread::sleep(Duration::from_millis(250));
    }
    diagnostics.record("Initialization completed successfully");
    drop(progress);
    {
        let _progress = ui::Progress::new(ui::text(
            "网络组件注入成功，强制局域网联机模式已开启，十秒后将关闭该窗口",
            "Network components injected successfully. Forced LAN mode is enabled. This window will close in ten seconds"));
        thread::sleep(Duration::from_secs(10));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_games_require_manual_shutdown() {
        for pids in [vec![123], vec![123, 456]] {
            let message = require_closed_game(&pids).unwrap_err().to_string();
            assert!(message.contains("Close all game instances manually"));
            assert!(message.contains("reopen this tool"));
        }
        assert!(require_closed_game(&[]).is_ok());
    }
}
