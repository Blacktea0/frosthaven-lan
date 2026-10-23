> **Antivirus false-positive warning**
> This tool injects DLLs into the game process and patches code in memory, so antivirus software may flag it. Official Release binaries are built by this repository's GitHub Actions from the corresponding source tag; this is not a guarantee of safety. If you are concerned about malware, read the source yourself or ask AI to help review it, then compile it on your own machine. If you are still uncomfortable, do not use this tool. Do not disable your antivirus to use it.

# Frosthaven LAN

[简体中文](README.md)

## Introduction

A Windows x64 LAN tool for Frosthaven. It enables the built-in ENet backend through in-memory injection without modifying installed game files. Currently supports **Retail 1.0.34121 Steam**.

See [PRODUCT.md](PRODUCT.md) for design details, advanced commands, and troubleshooting.

## How to use

1. Install and sign in to Steam, and confirm the game launches normally. Both players need the same game and patch version.
2. Download the ZIP from [Releases](https://github.com/Blacktea0/frosthaven-lan/releases/latest) and **extract all files** into a writable folder. Do not copy only the EXE.
3. Choose a startup mode: **double-click** `frosthaven-lan.exe` normally to launch the game EXE directly, or **right-click → Run as administrator** to launch the game through Steam immediately. An existing game process is reused.
4. Wait for `LAN backend ready`. The host loads a campaign and opens multiplayer from the pause menu.
5. The client selects Multiplayer → Join Game and enters the host IPv4 address (for example `192.168.1.20`, without a port). The host assigns characters and confirms.

**Due to Windows access restrictions, Steam-launched game processes require administrator permission for injection.** If the game is already running through Steam, close the tool and reopen it using Run as administrator. Standard permissions are intended for direct game startup by this tool.

Allow **UDP 30000** across the network. Other computers must use the host's actual IP rather than the `localhost` shown in the host UI. Steam identity initialization is still required; fully offline startup is unsupported.

After injection, press Enter to close the tool; the game keeps running. Check `launcher-error.log` in the extracted folder if injection fails. Restart the game after updating the patch.

## Development requirements

- Windows x64.
- Rust stable with the `x86_64-pc-windows-msvc` toolchain.
- Visual Studio C++ Build Tools with MSVC and the Windows SDK.
- Windows .NET Framework C# compiler: `C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe`.
- Windows PowerShell for ZIP packaging.

Harmony 2.3.3 is included. Building does not require the game; live injection tests require the supported game and Steam environment.

## Building and development

Run from the repository root. Close the game before rebuilding.

```text
cargo xtask build           # Build the production runtime in dist/
cargo xtask package         # Write production ZIP and SHA256 to artifacts/
cargo xtask run             # Build and run using the terminal's permission level
cargo xtask run --pid <PID> # Use an existing game process
```

There is one runtime: standard terminals launch the game directly; administrator terminals launch it through Steam. Use `xtask` for the complete runtime; `cargo build` alone builds only Rust components.

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo xtask test-managed
```

Use English [Conventional Commits](AGENTS.md), such as `fix: wait for runtime assemblies`.
