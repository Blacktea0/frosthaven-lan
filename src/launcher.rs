use super::config::APP_ID;
use super::{
    discovery, fingerprint_game, inject, ipc, modules, require_unchanged_game, trace, ui,
    win::{game_pids, is_elevated, open_steam_game},
    Owned, Result,
};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

const GAME_ALREADY_RUNNING: &str = "Frosthaven is already running. Close all game instances manually, then reopen this tool. Existing game processes will not be injected.";

struct GameProcess {
    handle: Owned,
    pid: u32,
}

impl GameProcess {
    fn open(pid: u32) -> Result<Self> {
        // Retain a handle before injection: a PID snapshot loses both the exit
        // code and process identity once the game has gone away.
        let handle = unsafe { super::win::OpenProcess(0x00101000, 0, pid) };
        if handle.is_null() {
            return Err(trace::context(
                format!("OpenProcess for startup monitoring failed; PID={pid}, access=0x101000"),
                std::io::Error::last_os_error(),
            ));
        }
        Ok(Self {
            handle: Owned(handle),
            pid,
        })
    }

    fn require_running(&self, stage: &'static str, diagnostics: &trace::Diagnostics) -> Result<()> {
        match unsafe { super::win::WaitForSingleObject(self.handle.0, 0) } {
            258 => Ok(()), // WAIT_TIMEOUT: the process is still running.
            0 => {
                let mut code = 0;
                if unsafe { super::win::GetExitCodeProcess(self.handle.0, &mut code) } == 0 {
                    return Err(trace::context(
                        stage,
                        trace::context(
                            format!("GetExitCodeProcess failed; PID={}", self.pid),
                            std::io::Error::last_os_error(),
                        ),
                    ));
                }
                diagnostics.record(format!(
                    "{stage}; PID={}; process exit code={code} (0x{code:08x})",
                    self.pid
                ));
                Err(stage.into())
            }
            u32::MAX => Err(trace::context(
                format!("WaitForSingleObject(process) failed; PID={}", self.pid),
                std::io::Error::last_os_error(),
            )),
            result => Err(format!(
                "Unexpected process wait result=0x{result:08x}; PID={}",
                self.pid
            )
            .into()),
        }
    }
}

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
    require_steam_login()?;
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

const STEAM_LOGIN_REQUIRED: &str = "Steam login could not be confirmed. Open Steam and finish signing in, then reopen this tool; alternatively run this tool as administrator to launch through Steam.";

fn steam_session_ready(pids: &[u32], active_pid: u32, active_user: u32) -> bool {
    active_pid != 0 && active_user != 0 && pids.contains(&active_pid)
}

fn require_steam_login() -> Result<()> {
    // ActiveProcess describes the current local client session, not server connectivity.
    // Match its PID to a live Steam process so stale registry values cannot pass alone.
    let pid = super::win::steam_active_dword("pid")
        .map_err(|error| trace::context(STEAM_LOGIN_REQUIRED, error))?;
    let user = super::win::steam_active_dword("ActiveUser")
        .map_err(|error| trace::context(STEAM_LOGIN_REQUIRED, error))?;
    let pids = super::win::pids_named("steam.exe")
        .map_err(|error| trace::context(STEAM_LOGIN_REQUIRED, error))?;
    if !steam_session_ready(&pids, pid, user) {
        return Err(STEAM_LOGIN_REQUIRED.into());
    }
    Ok(())
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
    let initial_hash = fingerprint_game(&game)?;
    diagnostics.record(format!("Game assembly SHA256: {initial_hash}; compatibility will be checked by managed structural discovery"));
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
            "Checking Steam local login session before starting game EXE directly"
        });
        wait_for_game(elevated, &game)?
    };
    diagnostics.record(format!("Detected game PID {pid}; validating process path"));
    let process = GameProcess::open(pid)?;
    diagnostics.connect(pid)?;
    diagnostics.record(format!(
        "Resolving Steam installation for detected game PID {pid}"
    ));
    let actual_game = discovery::for_process(pid).map_err(|error| {
        trace::context(
            format!("Validating detected game process failed; PID={pid}"),
            error,
        )
    })?;
    diagnostics.record(format!(
        "Process path validated; PID={pid}, installation={}",
        actual_game.display()
    ));
    diagnostics.record("Rechecking detected game's assembly SHA256");
    let hash = fingerprint_game(&actual_game).map_err(|error| {
        trace::context("Reading detected game's assembly fingerprint failed", error)
    })?;
    require_unchanged_game(&initial_hash, &hash)?;
    diagnostics.record(format!("Detected game assembly SHA256 unchanged: {hash}"));
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
    diagnostics.record(format!(
        "Waiting for Mono runtime; PID={pid}, timeout=120 seconds"
    ));
    let mut module_attempt = 0u32;
    loop {
        module_attempt += 1;
        process.require_running("Target game exited", diagnostics)?;
        let loaded = match unsafe { modules(pid) } {
            Ok(modules) => modules.contains_key("mono-2.0-bdwgc.dll"),
            Err(error) => {
                diagnostics.record(format!("Module enumeration failed; PID={pid}, attempt={module_attempt}, elapsed={:?}; {error}", Duration::from_secs(120).saturating_sub(deadline.saturating_duration_since(Instant::now()))));
                if trace::os_error_code(error.as_ref()) == Some(5) {
                    return Err(trace::context("Access to the game process was denied while reading modules; close the game and reopen this tool as administrator", error));
                }
                // Toolhelp can report ERROR_BAD_LENGTH or ERROR_PARTIAL_COPY
                // during early process/module initialization. Retry within the
                // startup deadline, retaining every original failure in memory.
                if matches!(trace::os_error_code(error.as_ref()), Some(24) | Some(299)) {
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
        if module_attempt == 1 || module_attempt % 20 == 0 {
            diagnostics.record(format!("Mono runtime not yet loaded; PID={pid}, module enumeration attempt={module_attempt}"));
        }
        if Instant::now() >= deadline {
            return Err("Timed out waiting for the game Mono runtime to initialize.".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
    unsafe {
        if let Err(error) = inject(pid, &initial_hash, diagnostics) {
            // Preserve the original injection error even if the game died in
            // an API call; append process state to the diagnostic history.
            if let Err(exit) = process.require_running("Game exited during injection", diagnostics)
            {
                diagnostics.record(format!("Injection process check: {exit:?}"));
            }
            return Err(error);
        }
    }
    let deadline = Instant::now() + Duration::from_secs(150);
    let mut last_channel_error = None;
    let mut last_state = None;
    loop {
        if let Err(error) = process.require_running("Game exited after injection", diagnostics) {
            diagnostics.record(format!("Last startup state: {last_state:?}; last shared-memory error: {last_channel_error:?}"));
            return Err(error);
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
    fn game_updates_are_accepted_but_changes_during_startup_are_refused() {
        let old = "20e5c317378426b0b3bf2833e4f54ed34677e062dafcaeecedd1b09340a606c5";
        let updated = "c16c02d74985fafdf5c02ab0c5d33ca5c5cf646be4b064474f8f40989e4ff13c";
        assert!(require_unchanged_game(old, old).is_ok());
        assert!(require_unchanged_game(updated, updated).is_ok());
        let error = require_unchanged_game(old, updated)
            .unwrap_err()
            .to_string();
        assert!(error.contains(old) && error.contains(updated));
    }
    #[test]
    fn process_monitor_retains_exit_code_after_process_disappears() {
        // Use a harmless child shell, never the game or injection path.
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "exit 37"])
            .spawn()
            .unwrap();
        // The Child handle keeps the process object alive even if it exits
        // before the monitor opens it.
        let process = GameProcess::open(child.id()).unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(37));
        let diagnostics = trace::Diagnostics::default();
        assert!(process
            .require_running("Game exited after injection", &diagnostics)
            .is_err());
        let report = diagnostics.records.snapshot().join("\n");
        assert!(report.contains("process exit code=37 (0x00000025)"));
    }

    #[test]
    fn process_monitor_accepts_running_process() {
        let process = GameProcess::open(std::process::id()).unwrap();
        process
            .require_running("Target game exited", &trace::Diagnostics::default())
            .unwrap();
    }

    #[test]
    fn steam_login_requires_live_matching_client_and_active_user() {
        assert!(steam_session_ready(&[12], 12, 42));
        assert!(!steam_session_ready(&[], 12, 42));
        assert!(!steam_session_ready(&[13], 12, 42));
        assert!(!steam_session_ready(&[12], 12, 0));
        assert!(!steam_session_ready(&[0], 0, 42));
    }
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
