//! Workspace tasks. Programs are executed directly; no command shell is involved.
#[path = "../../src/config.rs"]
mod config;
#[path = "../../src/discovery.rs"]
#[allow(dead_code)]
mod discovery;
#[path = "../../src/win.rs"]
mod win;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn run(mut command: Command) -> Result<()> {
    let label = command.get_program().to_string_lossy().into_owned();
    let status = command.status()?;
    if !status.success() {
        return Err(format!("{label} failed: {status}").into());
    }
    Ok(())
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn build(root: &Path) -> Result<()> {
    if !win::game_pids()?.is_empty() {
        return Err("Close Frosthaven before rebuilding loaded DLLs".into());
    }
    let csc = PathBuf::from(env::var_os("WINDIR").ok_or("WINDIR is missing")?)
        .join(r"Microsoft.NET\Framework64\v4.0.30319\csc.exe");
    if !csc.is_file() {
        return Err(format!("C# compiler not found: {}", csc.display()).into());
    }
    let dist = root.join("dist");
    println!("Supported game assembly SHA256: {}", config::HASH);
    fs::create_dir_all(&dist)?;
    for (source, name) in [
        ("README.md", "README.md"),
        ("README.en.md", "README.en.md"),
        ("PRODUCT.md", "PRODUCT.md"),
        ("vendor/harmony/LICENSE", "Harmony-LICENSE.txt"),
    ] {
        fs::copy(root.join(source), dist.join(name))?;
    }
    fs::copy(
        root.join("vendor/harmony/0Harmony.dll"),
        dist.join("0Harmony.dll"),
    )?;
    for (output, sources) in [
        ("Bootstrap.dll", vec!["managed/Bootstrap.cs"]),
        (
            "LanPayload.dll",
            vec![
                "managed/Payload.cs",
                "managed/LanText.cs",
                "managed/RuntimeAssemblies.cs",
            ],
        ),
    ] {
        let mut command = Command::new(&csc);
        command
            .current_dir(root)
            .args(["/nologo", "/utf8output", "/platform:x64"])
            .arg("/target:library")
            .arg(format!("/out:{}", dist.join(output).display()));
        if output == "LanPayload.dll" {
            command.arg(format!("/r:{}", dist.join("0Harmony.dll").display()));
        }
        for source in sources {
            // Framework csc interprets relative forward slashes as options and
            // cannot read canonicalize()'s Windows verbatim (\\?\) paths.
            command.arg(root.join(source.replace('/', "\\")));
        }
        run(command)?;
    }
    let mut cargo = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    let target = root.join("target");
    cargo
        .current_dir(root)
        .env("CARGO_TARGET_DIR", &target)
        .args(["build", "--locked", "--package", "frosthaven-lan"]);
    cargo.arg("--release");
    run(cargo)?;
    fs::copy(
        target.join("release/lanbridge.dll"),
        dist.join("LanBridge.dll"),
    )?;
    fs::copy(
        target.join("release/frosthaven-lan.exe"),
        dist.join("frosthaven-lan.exe"),
    )?;
    println!("Complete runtime built in {}", dist.display());
    Ok(())
}
fn test_managed(root: &Path) -> Result<()> {
    let csc = PathBuf::from(env::var_os("WINDIR").ok_or("WINDIR is missing")?)
        .join(r"Microsoft.NET\Framework64\v4.0.30319\csc.exe");
    fs::create_dir_all(root.join("target"))?;
    let output = root.join("target/RuntimeAssembliesTests.exe");
    let mut compiler = Command::new(csc);
    compiler
        .args(["/nologo", "/utf8output"])
        .arg(format!("/out:{}", output.display()))
        .arg(root.join(r"managed\RuntimeAssemblies.cs"))
        .arg(root.join(r"tools\tests\RuntimeAssembliesTests.cs"));
    run(compiler)?;
    run(Command::new(output))
}
fn package(root: &Path) -> Result<()> {
    build(root)?;
    let mut command = Command::new("powershell.exe");
    command.current_dir(root).args([
        "-NoProfile",
        "-NonInteractive",
        "-File",
        "tools/package.ps1",
    ]);
    run(command)
}
fn tool(root: &Path, command: &str, args: &[String]) -> Result<()> {
    let exe = root.join("dist/frosthaven-lan.exe");
    if !exe.is_file() {
        return Err("Run cargo xtask build first".into());
    }
    let mut process = Command::new(exe);
    process.current_dir(root).arg(command).args(args);
    run(process)
}
fn main() {
    let result = (|| -> Result<()> {
        let root = root();
        let args: Vec<String> = env::args().skip(1).collect();
        match args.first().map(String::as_str) {
            Some("build") if args.len() == 1 => build(&root),
            Some("test-managed") if args.len() == 1 => test_managed(&root),
            Some("package") if args.len() == 1 => package(&root),
            Some("run") => {
                if win::game_pids()?.is_empty() {
                    build(&root)?;
                }
                tool(&root, "start", &args[1..])
            }
            Some("verify") if args.len() == 1 => tool(&root, "verify", &[]),
            Some("locate") if args.len() == 1 => {
                println!("{}", discovery::locate()?.display());
                Ok(())
            }
            Some("command") if args.len() > 1 => tool(&root, "command", &args[1..]),
            None | Some("help" | "--help" | "-h") => {
                println!("cargo xtask build\ncargo xtask test-managed\ncargo xtask package\ncargo xtask run [--pid PID]\ncargo xtask locate\ncargo xtask verify\ncargo xtask command <status|steam-status|restore>");
                Ok(())
            }
            _ => Err("Unknown task or invalid arguments; use cargo xtask help".into()),
        }
    })();
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
