> **Antivirus false-positive warning**
> This tool injects DLLs into the game process and patches code in memory, so antivirus software may flag it. Official Release binaries are built by this repository's GitHub Actions from the corresponding source tag; this is not a guarantee of safety. If you are concerned about malware, read the source yourself or ask AI to help review it, then compile it on your own machine. If you are still uncomfortable, do not use this tool. Do not disable your antivirus to use it.

# Frosthaven LAN

[简体中文](README.md)

Windows x64 LAN injection tool for Frosthaven. It switches the network backend to built-in ENet while retaining game sessions, player checks, and synchronization. Patches apply only in memory; game files, Steam DLLs, and language resources are not replaced.

Rust handles launching, injection, Steam discovery, and Mono bridging. The C# payload uses Harmony and resolves game types at runtime. Application logs and messages are in English; game text supports all 12 languages.

## Requirements

- Windows x64, Steam installed and signed in, and Frosthaven able to launch through Steam.
- Supported game: Retail 1.0.34121 Steam. Other versions are rejected.
- Administrator permission: the executable embeds `requireAdministrator`. Approve Windows UAC, or use an administrator terminal for CLI commands.

Supported `Assembly-CSharp.dll` SHA256:

```text
20e5c317378426b0b3bf2833e4f54ed34677e062dafcaeecedd1b09340a606c5
```

The injector checks the process against Steam's registered installation and verifies this hash. Steam libraries and custom installation paths are discovered automatically.

## Download and run

Download the ZIP from GitHub Releases and extract **all files** into a writable folder. Double-click `frosthaven-lan.exe` and approve UAC. Keep these five runtime files together:

```text
frosthaven-lan.exe
LanBridge.dll
Bootstrap.dll
LanPayload.dll
0Harmony.dll
```

The archive also includes both READMEs and the Harmony license. Rust is not required to run it.

When the game is not running, start it through Steam yourself or press Enter to request startup via `steam://rungameid/2347080`. The tool waits and injects automatically once detected; close its window to cancel. A requested Steam startup times out after three minutes. The game EXE is never launched directly. Existing game processes are reused; an already enabled patch is reported without another injection.

After injection, press Enter to close the console; the game keeps running. Failures are written to `launcher-error.log`. Restart the game after updating the payload or restoring the original backend. Use `--pid` when multiple instances are running, with separate extracted runtime folders to avoid sharing command files.

## Multiplayer

1. Both players use the same supported game and patch version, with network connectivity.
2. The host loads a campaign and opens multiplayer from the pause menu.
3. The client selects Multiplayer → Join Game and enters the host IPv4 address, such as `192.168.1.20`, without a port.
4. The host assigns characters and confirms entry.

The game uses **UDP 30000**. The host UI currently shows `localhost`; other computers must use the actual host IP. Steam identity initialization remains required; fully offline startup is unsupported.

## Build and package

Install Rust stable for `x86_64-pc-windows-msvc` and Visual Studio C++ Build Tools. The build uses the Windows .NET Framework compiler at `C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe`. Harmony 2.3.3 and its license are included in `vendor/harmony/`. No installed game is needed to build. Close the game before rebuilding loaded DLLs.

```text
cargo xtask build
cargo xtask package
cargo xtask run
cargo xtask run --pid <PID>
```

`build` produces `dist/`. `package` builds and creates `artifacts/frosthaven-lan-v<version>-windows-x64.zip` and a SHA256 checksum. Packaging verifies extracted contents, hashes, and the administrator manifest. Only explicitly listed runtime and documentation files are included. The native C runtime is statically linked.

`run` builds when no game process exists, then waits for startup. With a running game it uses the existing package. Use an administrator terminal. `cargo build --release --locked` alone builds only Rust components.

## Commands and recovery

```text
.\dist\frosthaven-lan.exe
.\dist\frosthaven-lan.exe start --pid <PID>
.\dist\frosthaven-lan.exe inject <PID>
cargo xtask locate
cargo xtask verify
cargo xtask command status
cargo xtask command steam-status
cargo xtask command restore
```

`locate` shows the Steam installation; `verify` checks its game assembly. Payload command results are in `dist/payload.log`, native diagnostics in `dist/native.log`, bootstrap failures in `dist/bootstrap.log`. Game logs use their normal location.

End multiplayer before `restore`, which restores the original backend, removes method patches, and refreshes the UI. Loaded DLLs and input settings are fully released on game exit. The game continues writing normal configuration and saves.

## Localization

The patch replaces the invite-code prompt with a host-IP prompt and the hosting confirmation with a LAN confirmation. It follows the active game language: English, German, French, Italian, Spanish, Brazilian Portuguese, Hungarian, Russian, simplified/traditional Chinese, Japanese, and Korean. Language files remain unchanged.

## Releases and contributing

Push a `v*` tag matching `Cargo.toml` (for example `v0.2.0`) to trigger a Windows GitHub Actions build, archive verification, and GitHub Release containing the ZIP and checksum. Release notes repeat the antivirus warning. Build logs are available in Actions.

Use English Conventional Commits, such as `feat: add release packaging`. See [AGENTS.md](AGENTS.md).

## Source layout

- `src/`: Rust CLI, launcher, Steam discovery, Mono bridge, Windows API.
- `managed/`: C# patches and game text.
- `xtask/`: build tasks.
- `tools/`: packaging and read-only analysis helpers.
- `docs/reverse-engineering/`: analysis and evidence.
- `vendor/harmony/`: Harmony DLL, license, provenance.
- `target/`, `dist/`, `artifacts/`, `logs/`: ignored generated files.

[Analysis notes (Chinese)](docs/reverse-engineering/README.md) · [Harmony provenance](vendor/harmony/README.md)
