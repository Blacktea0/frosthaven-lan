use std::{
    io::{self, IsTerminal, Write},
    sync::{mpsc, OnceLock},
    thread,
    time::Duration,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Locale {
    Zh,
    En,
}
impl Locale {
    pub fn from_name(name: Option<&str>) -> Self {
        match name {
            None | Some("") => Self::Zh,
            Some(s) if s.to_ascii_lowercase().starts_with("zh") => Self::Zh,
            _ => Self::En,
        }
    }
}
pub fn locale() -> Locale {
    static VALUE: OnceLock<Locale> = OnceLock::new();
    *VALUE.get_or_init(|| Locale::from_name(super::win::user_locale().as_deref()))
}
pub fn text<'a>(zh: &'a str, en: &'a str) -> &'a str {
    if locale() == Locale::Zh {
        zh
    } else {
        en
    }
}
/// Reserve the spinner column for completed/static startup messages too.
pub fn line(message: impl std::fmt::Display) {
    if io::stdout().is_terminal() {
        println!("  {message}");
    } else {
        println!("{message}");
    }
}
pub struct Progress {
    stop: Option<mpsc::Sender<()>>,
    worker: Option<thread::JoinHandle<()>>,
    message: String,
    interactive: bool,
}
impl Progress {
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let interactive = io::stdout().is_terminal();
        if !interactive {
            line(&message);
            return Self {
                stop: None,
                worker: None,
                message,
                interactive,
            };
        }
        let (tx, rx) = mpsc::channel();
        let label = message.clone();
        let worker = thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            loop {
                print!("\r{} {}", frames[i % frames.len()], label);
                let _ = io::stdout().flush();
                if rx.recv_timeout(Duration::from_millis(100))
                    != Err(mpsc::RecvTimeoutError::Timeout)
                {
                    break;
                }
                i += 1;
            }
        });
        Self {
            stop: Some(tx),
            worker: Some(worker),
            message,
            interactive,
        }
    }
}
impl Drop for Progress {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if self.interactive {
            print!("\r");
            line(&self.message);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locale_fallbacks() {
        assert_eq!(Locale::from_name(None), Locale::Zh);
        for s in ["", "zh-CN", "zh-TW", "zh-Hans"] {
            assert_eq!(Locale::from_name(Some(s)), Locale::Zh);
        }
        for s in ["en-US", "en-GB", "fr-FR", "ja-JP"] {
            assert_eq!(Locale::from_name(Some(s)), Locale::En);
        }
    }
}

pub fn diagnostic(message: &str) -> String {
    if locale() == Locale::En {
        return message.to_owned();
    }
    let translated = match message.trim() {
        "Frosthaven is already running. Close all game instances manually, then reopen this tool. Existing game processes will not be injected." => "游戏已在运行，请手动关闭所有游戏实例后重新打开本工具。",
        "Multiple game instances detected. Close all game instances manually, then reopen this tool." => "检测到多个游戏实例，请手动关闭所有实例后重新打开本工具。",
        "Steam startup timed out. Check Steam and reopen this tool." => "Steam 启动超时，请检查 Steam，并在关闭游戏后重新打开本工具。",
        _ if message.contains("Steam login could not be confirmed.") => "无法确认 Steam 已登录。请先打开 Steam 并完成登录，再重新打开本工具；也可右键以管理员身份运行，通过 Steam 启动游戏。",
        "Direct game startup timed out" => "游戏启动超时，请关闭游戏后重新打开本工具。",
        "Target game exited" | "Game exited after injection" => "游戏已退出，请重新打开本工具。",
        _ if message.starts_with("Game assembly changed during startup") => "游戏文件在启动期间发生变化，请关闭游戏，等待更新完成后重新打开本工具。",
        "Timed out waiting for the game Mono runtime to initialize." => "等待游戏运行时初始化超时，请关闭游戏后重新打开本工具。",
        "DLL did not load" => "网络组件加载失败，请关闭游戏后重新打开本工具。",
        "LoadLibrary timeout; remote buffer retained" => "网络组件加载超时，请关闭游戏后重新打开本工具。",
        "Already injected. Close the game manually, then reopen this tool." => "游戏已经注入过组件，请手动关闭游戏后重新打开本工具。",
        "Frosthaven was not found in Steam libraries. Install the game through Steam first." => "未找到游戏，请先通过 Steam 安装 Frosthaven。",
        "Multiple Frosthaven installations found. Keep only the intended Steam installation registered before reopening this tool." => "检测到多个 Frosthaven 安装，请只保留要使用的 Steam 游戏安装记录后重新打开本工具。",
        "The target process does not belong to a Frosthaven installation registered with Steam." => "游戏进程与 Steam 登记的安装目录不匹配，请关闭游戏后重新打开本工具。",
        "Mono missing" => "游戏运行时尚未就绪，请关闭游戏后重新打开本工具。",
        "Bootstrap load failed" | "Managed entry failed" => "网络组件入口加载失败，请完整解压运行包，并在关闭游戏后重新打开本工具。",
        "Mono root domain initialization timed out" | "Mono root domain and main thread initialization timed out" | "Mono thread attachment failed" => "无法初始化游戏运行时，请关闭游戏后重新打开本工具。",
        _ if message.starts_with("Direct game startup exited:") => "游戏在启动期间退出，请检查游戏和 Steam 状态后重试。",
        _ if message.starts_with("BOOTSTRAP ERROR ") || message.starts_with("INSTALL ERROR ") || message.starts_with("PUMP ERROR ") => return format!("网络组件初始化失败：{message}"),
        _ if message.starts_with("STATUS backend=") => return message.replace("STATUS backend=", "当前网络后端：").replace(" running=", "，会话运行状态："),
        _ if message.starts_with("RESTORED;") => "已恢复原网络后端，请关闭游戏后重新打开工具以再次启用局域网模式。",
        _ if message.contains("Access to the game process was denied") => "无法访问游戏进程，请关闭游戏后以管理员身份重新打开本工具。如需 Steam 游戏内叠加界面，请使用管理员模式启动。",
        _ if message.starts_with("Missing ") && message.contains("cargo xtask build") => "运行文件不完整，请完整解压 ZIP，或执行 cargo xtask build 重新编译。",
        _ if message.starts_with("Usage:") => "用法：frosthaven-lan [start | command <status|steam-status|restore> | locate | verify]",
        _ => return message.to_owned(),
    };
    translated.to_owned()
}

pub fn pause_after_failure() {
    println!(
        "{}",
        text("按 Enter 关闭窗口。", "Press Enter to close this window.")
    );
    let _ = io::stdin().read_line(&mut String::new());
}
