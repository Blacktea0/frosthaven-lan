//! Read Steam's registry entries, library folders and app manifests. Never writes to Steam.
use crate::{config::APP_ID, win};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
enum Value {
    Text(String),
    Object(Vec<(String, Value)>),
}
fn tokens(text: &str) -> Option<Vec<String>> {
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut out = Vec::new();
    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            for ch in chars.by_ref() {
                if ch == '\n' {
                    break;
                }
            }
            continue;
        }
        if ch == '{' || ch == '}' {
            out.push(ch.to_string());
            continue;
        }
        let mut token = String::new();
        if ch == '"' {
            let mut closed = false;
            while let Some(ch) = chars.next() {
                if ch == '"' {
                    closed = true;
                    break;
                }
                if ch == '\\' && matches!(chars.peek(), Some('\\' | '"')) {
                    token.push(chars.next()?);
                } else {
                    token.push(ch);
                }
            }
            if !closed {
                return None;
            }
        } else {
            token.push(ch);
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() || ch == '{' || ch == '}' {
                    break;
                }
                token.push(chars.next()?);
            }
        }
        out.push(token);
    }
    Some(out)
}
fn object(tokens: &[String], index: &mut usize, nested: bool) -> Option<Vec<(String, Value)>> {
    let mut out = Vec::new();
    while *index < tokens.len() {
        let key = tokens.get(*index)?.clone();
        *index += 1;
        if key == "}" {
            return nested.then_some(out);
        }
        if key == "{" {
            return None;
        }
        let value = tokens.get(*index)?.clone();
        *index += 1;
        let value = if value == "{" {
            Value::Object(object(tokens, index, true)?)
        } else if value == "}" {
            return None;
        } else {
            Value::Text(value)
        };
        out.push((key, value));
    }
    (!nested).then_some(out)
}
fn read(path: &Path) -> Option<Vec<(String, Value)>> {
    let text = fs::read_to_string(path).ok()?;
    let tokens = tokens(&text)?;
    object(&tokens, &mut 0, false)
}
fn child<'a>(entries: &'a [(String, Value)], name: &str) -> Option<&'a [(String, Value)]> {
    entries.iter().find_map(|(key, value)| match value {
        Value::Object(data) if key.eq_ignore_ascii_case(name) => Some(data.as_slice()),
        _ => None,
    })
}
fn string<'a>(entries: &'a [(String, Value)], name: &str) -> Option<&'a str> {
    entries.iter().find_map(|(key, value)| match value {
        Value::Text(data) if key.eq_ignore_ascii_case(name) => Some(data.as_str()),
        _ => None,
    })
}
fn add(paths: &mut Vec<PathBuf>, seen: &mut HashSet<String>, path: PathBuf) {
    if path.is_dir() && seen.insert(path.to_string_lossy().to_lowercase()) {
        paths.push(path);
    }
}
pub fn same_path(a: &Path, b: &Path) -> bool {
    fn normalize(path: &Path) -> String {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        path.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('/', "\\")
            .to_lowercase()
    }
    normalize(a) == normalize(b)
}
pub fn installed() -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    if let Some(path) = win::registry_string(false, r"Software\Valve\Steam", "SteamPath", 0) {
        add(&mut roots, &mut seen, PathBuf::from(path));
    }
    for view in [0, 0x10000, 0x20000] {
        if let Some(path) = win::registry_string(true, r"SOFTWARE\Valve\Steam", "InstallPath", view)
        {
            add(&mut roots, &mut seen, PathBuf::from(path));
        }
    }
    for pid in win::pids_named("steam.exe")? {
        if let Ok(path) = win::process_path(pid) {
            if let Some(parent) = path.parent() {
                add(&mut roots, &mut seen, parent.to_path_buf());
            }
        }
    }
    for name in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(path) = std::env::var_os(name) {
            add(&mut roots, &mut seen, PathBuf::from(path).join("Steam"));
        }
    }
    let mut libraries = roots.clone();
    for root in roots {
        for relative in ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"] {
            if let Some(data) = read(&root.join(relative)) {
                if let Some(folders) = child(&data, "libraryfolders") {
                    for (key, value) in folders {
                        if key.parse::<u32>().is_err() {
                            continue;
                        }
                        let path = match value {
                            Value::Text(path) => Some(path.as_str()),
                            Value::Object(data) => string(data, "path"),
                        };
                        if let Some(path) = path {
                            add(&mut libraries, &mut seen, PathBuf::from(path));
                        }
                    }
                }
            }
        }
    }
    let mut games: Vec<PathBuf> = Vec::new();
    for library in libraries {
        let Some(data) = read(&library.join(format!("steamapps/appmanifest_{APP_ID}.acf"))) else {
            continue;
        };
        let Some(app) = child(&data, "AppState") else {
            continue;
        };
        if string(app, "appid") != Some(APP_ID) {
            continue;
        }
        let Some(folder) = string(app, "installdir") else {
            continue;
        };
        if folder.is_empty()
            || !Path::new(folder)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        {
            continue;
        }
        let game = library.join("steamapps/common").join(folder);
        if game.join("Frosthaven.exe").is_file()
            && game
                .join("Frosthaven_Data/Managed/Assembly-CSharp.dll")
                .is_file()
            && !games.iter().any(|p| same_path(p, &game))
        {
            let canonical = game.canonicalize()?;
            let path = canonical.to_string_lossy();
            let path = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
                PathBuf::from(format!(r"\\{unc}"))
            } else {
                PathBuf::from(path.trim_start_matches(r"\\?\"))
            };
            games.push(path);
        }
    }
    Ok(games)
}
pub fn locate() -> Result<PathBuf> {
    let mut games = installed()?;
    match games.len() {
        0 => Err("Frosthaven was not found in Steam libraries. Install the game through Steam first.".into()),
        1 => Ok(games.remove(0)),
        _ => Err("Multiple Frosthaven installations found. Keep only the intended Steam installation registered before reopening this tool.".into()),
    }
}
pub fn for_process(pid: u32) -> Result<PathBuf> {
    let exe = win::process_path(pid)?;
    for game in installed()? {
        if same_path(&exe, &game.join("Frosthaven.exe")) {
            return Ok(game);
        }
    }
    Err(
        "The target process does not belong to a Frosthaven installation registered with Steam."
            .into(),
    )
}
