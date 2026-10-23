> [!CAUTION]
> **Antivirus false-positive warning**
> This tool injects DLLs into the game process and patches code in memory, so antivirus software may flag it. Official Release binaries are built by this repository's GitHub Actions from the corresponding source tag; this is not a guarantee of safety. If you are concerned about malware, read the source yourself or ask AI to help review it, then compile it on your own machine. If you are still uncomfortable, do not use this tool. Do not disable your antivirus to use it.

# Frosthaven LAN

[简体中文](README.md)

## Introduction

A Windows x64 LAN tool for Frosthaven. It enables the built-in ENet backend through in-memory injection without modifying installed game files. Currently supports **Retail 1.0.34121 Steam**.

When enabled, the main menu version label ends with `LAN Enabled`, and invite-code UI text uses the host IP instead.

See [PRODUCT.md](PRODUCT.md) for design details, advanced commands, and troubleshooting.

## How to use

1. Install and sign in to Steam, and confirm the game launches normally. Both players need the same game and patch version.
2. Download the ZIP from [Releases](https://github.com/Blacktea0/frosthaven-lan/releases/latest) and **extract all files** into a writable folder. Do not copy only the EXE.
3. Choose a startup mode: **double-click** `frosthaven-lan.exe` normally to launch the game EXE directly, or **right-click → Run as administrator if you need the Steam in-game overlay**, which launches the game through Steam. **The game must be started by this tool.** If it is already running, close all game instances manually, then reopen the tool.
4. Wait for the message confirming successful injection and forced LAN mode. The host loads a campaign and opens multiplayer from the pause menu.
5. The client selects Multiplayer → Join Game and enters the host IPv4 address (for example `192.168.1.20`, without a port). The host assigns characters and confirms.

To enable the **Steam in-game overlay**, launch this tool as administrator. Standard mode starts the game EXE directly; both modes require the tool to start the game.

Allow **UDP 30000** across the network. Other computers must use the host's actual IP rather than the `localhost` shown in the host UI. Steam identity initialization is still required; fully offline startup is unsupported.

Startup stages use an animated spinner. Console language follows the Windows locale: Chinese locales use simplified Chinese, all other locales use English, and failed locale detection defaults to simplified Chinese. On success, the tool exits after ten seconds while the game keeps running. On failure, the console displays the full original error, inner exceptions, stack traces, Windows error codes, and timestamped diagnostic history. Every invocation waits for Enter after failure instead of closing automatically. The tool does not generate log files. After updating the patch, close the game and reopen the tool.

## Development requirements

- Windows x64.
- Rust stable, using your configured default toolchain with either setup:
  - MSVC: `x86_64-pc-windows-msvc` plus Visual Studio C++ Build Tools and the Windows SDK (including `kernel32.lib`).
  - GNU: `x86_64-pc-windows-gnu` plus MinGW-w64/GCC, with `gcc` and `windres` on PATH.
- Windows .NET Framework C# compiler: `C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe`.
- Windows PowerShell for ZIP packaging.

Harmony 2.3.3 is included. Building does not require the game; live injection tests require the supported game and Steam environment.

## Building and development

Run from the repository root. Close the game before rebuilding.

```text
cargo xtask build           # Build the production runtime in dist/
cargo xtask package         # Write production ZIP and SHA256 to artifacts/
cargo xtask run             # Build and run using the terminal's permission level
```

There is one runtime: standard terminals launch the game directly; administrator terminals launch it through Steam. Use `xtask` for the complete runtime; `cargo build` alone builds only Rust components.

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo xtask test-managed
```

Use English [Conventional Commits](AGENTS.md), such as `fix: wait for runtime assemblies`.
