# Frosthaven LAN: Design and Operation

[English usage guide](README.md) · [Simplified Chinese usage guide](README.zh-CN.md)

This document is maintained in the repository and is not included in release ZIPs.

## Purpose and boundaries

Enable Frosthaven's built-in ENet backend while retaining the game's existing sessions, player checks, synchronization, and save handling. Patches affect process memory only; they do not replace installed game assemblies, Steam DLLs, or language resources. The tool does not bypass Steam login or ownership checks. Normal game configuration, autosave, and game log writes continue.

## Architecture and startup

1. The Rust launcher discovers Steam through the registry, running client paths, `libraryfolders.vdf`, and `appmanifest_2347080.acf`. Custom paths and multiple libraries are supported.
2. It verifies that the target process belongs to a registered installation, records the game assembly SHA256, and checks that it remains unchanged during startup and immediately before injection. There is no fixed hash allowlist.
3. It loads `LanBridge.dll`, waits for both the Mono root domain and managed main thread object, then attaches the worker thread to Mono.
4. The bridge invokes `Bootstrap.dll`, which resolves packaged dependencies and loads `LanPayload.dll`.
5. The payload waits for `Assembly-CSharp` and `UnityEngine.CoreModule`, discovers compatible patch sites, and installs Harmony patches. Finding the Mono DLL alone does not mean these assemblies are ready.
6. The game-thread pump switches the backend, refreshes localized UI and input settings, and reports LAN readiness. An active network session prevents switching.

The native bridge waits up to 120 seconds for the root domain and main thread object. The managed layer waits up to 120 seconds for required assemblies. The launcher waits up to 150 seconds after injection for backend readiness. Errors and timeouts use memory IPC and console output. A loaded DLL alone does not prove that LAN mode is ready.

## Compatibility checks

The original support baseline was **Retail 1.0.34121 Steam**, with this historical `Assembly-CSharp.dll` fingerprint:

```text
20e5c317378426b0b3bf2833e4f54ed34677e062dafcaeecedd1b09340a606c5
```

Updated versions undergo runtime structural discovery:

- Search `FFSNetwork` and its nested types for exactly one EOS constructor call. The next non-NOP instruction must store the result in the compatible static `_backend` field. ENet must expose a constructor with exactly the same parameter types; reflection binder conversions are not accepted.
- Find the ENet connection method through its `ENet.Host.Connect(Address)` call and validate the two-argument channel-count overload.
- Find the avatar coroutine through its GPU copy flow and exactly one field referencing the owning player. Retain the existing resize, copy-flow, and exception-boundary guards.

Each target must be unique and pass its signature and IL checks. Discovery does not depend on metadata tokens, coroutine indices, or these method names. Backend type names, network properties, and UI interfaces must still be compatible. The tool does not search arbitrary assemblies for approximate replacements or relax protocol and avatar-copy requirements.

Actual Harmony instruction streams are checked before patching. Other patches are installed before the game-thread pump. Constructor replacement and the pump remain inactive until installation completes, so partial installation retains EOS construction. Failure disables activation and attempts to remove this tool's patches; rollback failures remain in diagnostics.

SHA256 records identify the assembly and detect file changes during startup. They do not prove protocol or multiplayer compatibility.

On 2026-10-28, read-only metadata/IL inspection of the updated assembly with fingerprint `c16c02d74985fafdf5c02ab0c5d33ca5c5cf646be4b064474f8f40989e4ff13c` found compatible backend construction and assignment, ENet connection, avatar-copy, and existing UI entry structures. This inspection did not start or inject into the game and did not validate actual startup or multiplayer behavior.

Subsequent authorized manual testing on the same date used an isolated temporary two-child harness, the same Steam account, and loopback address `127.0.0.1`. Both instances completed Mono attachment, structural discovery, patch installation, and LAN readiness. A separate `LAN-TEST-20261028` campaign covered hosting, joining, character assignment, battle-goal selection, readiness, shared battlefield loading, and first-round ability selection with matching initiative order. Both backends reported `running=True`; local and remote avatars were 184×184 with one mip and matching PNG hashes. Closing the client produced the expected disconnect notification on the host, and both test instances were closed afterward. The production launcher's existing-instance rejection was unchanged.

The first two-child attempt exposed a transient Toolhelp snapshot error 24 during pre-injection enumeration. Injection module snapshots now retry OS errors 24 and 299 for up to ten seconds, retaining every raw failure; access denial and other errors still fail immediately. Only enumeration is retried, never the remote DLL-loading operation. Regression tests cover recovery and error preservation; the subsequent manual attempt initialized both instances successfully.

This local test did not cover a complete combat round, long-running sessions, different Steam accounts, different computers, firewall/NAT behavior, mixed game versions, administrator-mode overlay behavior, or repeated cold-start stress. It is not a substitute for independent multi-machine validation.

## Startup permissions and Steam overlay

There is one runtime build in `dist/`; Cargo outputs remain under `target/`. The executable uses an `asInvoker` manifest, preserves caller permissions, and never requests elevation automatically. It checks the effective token's `TokenElevation` rather than assuming an administrator account is elevated.

| Launcher permissions | Behavior when the game is closed | How to use |
| --- | --- | --- |
| Standard | Start the registered `Frosthaven.exe` directly with `SteamAppId` | Double-click or use a standard terminal |
| Administrator | Request startup immediately through `steam://rungameid/2347080` | Select Run as administrator or use an elevated terminal |

Use administrator mode if you need the **Steam in-game overlay**. Close the game first and let the tool start it through Steam. Standard mode checks the current local Steam login session before launching; this does not guarantee Steam service connectivity or avatar availability. Game-side Steam authentication and ownership checks still apply. A lower-privilege launcher cannot access a higher-privilege game process.

A Steam startup request times out after three minutes if no game process appears. Both modes begin without an Enter prompt. Successful startup exits after ten seconds; every failed invocation waits for Enter, including command-line invocations.

The launcher checks for existing games at startup and immediately before launching. Existing instances are rejected with instructions to close them manually and reopen the tool. It does not terminate, inject into, or take over existing games. There is no PID selection or direct injection command. Standard mode targets only its newly created child. Steam mode waits for a game after its own launch request and rejects multiple instances. After updating the patch or restoring the original backend, close the game and reopen the tool to enable LAN again.

## Runtime package and builds

Keep these five runtime files in the same directory:

```text
frosthaven-lan.exe
LanBridge.dll
Bootstrap.dll
LanPayload.dll
0Harmony.dll
```

Release ZIPs also contain the English `README.md`, simplified Chinese `README.zh-CN.md`, and `Harmony-LICENSE.txt`. `PRODUCT.md` and `AGENTS.md` stay in the repository. README links to this document use its repository URL so they work from an extracted ZIP.

Rust handles launching, injection, the Mono bridge, and build tasks. C# resolves game types through runtime reflection, so builds do not require installed game assemblies. Official CI builds use Windows x64 MSVC. Local builds support the configured MSVC or GNU toolchain and statically link the native C runtime. MSVC requires Visual Studio C++ Build Tools and Windows SDK link libraries, including `kernel32.lib`. GNU requires GCC and `windres` on PATH. Close the game before rebuilding loaded DLLs. End users do not need Rust.

`cargo xtask package` builds the release runtime and archives an explicit allowlist from `dist/`. It excludes repository-only documentation, stale files, and runtime state files. Outputs are:

```text
artifacts/frosthaven-lan-v<version>-windows-x64.zip
artifacts/frosthaven-lan-v<version>-windows-x64.zip.sha256
```

All ZIP entries are under one top-level `frosthaven-lan/` directory. Packaging extracts the archive, verifies its directory layout, exact file list, and content hashes, and reads the executable's embedded manifest to confirm `asInvoker` and `uiAccess=false`.

## Advanced commands and restore

Commands use the current process permissions. Use an administrator terminal when working with a game launched through Steam:

```text
.\dist\frosthaven-lan.exe start
cargo xtask locate
cargo xtask verify
cargo xtask command status
cargo xtask command steam-status
cargo xtask command restore
```

- `start` requires the game to be closed, launches it, waits for Mono, injects, and confirms backend readiness.
- `locate` prints the registered Steam installation directory. `verify` prints the game assembly fingerprint; patch compatibility is checked during startup.
- `status` reports the current backend, session state, avatar dimensions, mipmap state, and PNG SHA256. `steam-status` reports Steam identity and overlay state when available.
- `restore` requires multiplayer to have ended. It restores the original network backend, disables LAN activation, removes method patches, and refreshes UI. Injected DLLs and input-instance settings are fully released when the game exits.

## Console, locale, and memory IPC

Chinese Windows user locales use simplified Chinese console output. Other locales use English; failed locale detection defaults to simplified Chinese. Waiting stages use animated spinners in a terminal. Redirected output emits one static line per stage without animation control characters.

Startup displays five stages: permission mode, game directory, game launch, detected process and network initialization, and successful LAN activation with the ten-second exit delay. Technical diagnostics are hidden on the normal success path.

The native bridge creates a process-lifetime memory mapping named `Local\FrosthavenLan-<PID>`. Native and managed components share separate slots for startup status, command requests, and command responses. Sequence numbers prevent partial reads. Each command has a unique identifier, and the calling console displays only a matching response.

Full diagnostic records use a separate named pipe, `FrosthavenLan-Diagnostics-<PID>`, rather than the limited shared-memory control slots. Records are framed by UTF-8 byte length and acknowledged only after the complete record has been retained. The pipe permits only the same Windows user and SYSTEM, with a medium integrity label to allow a standard game to report to the same user's elevated launcher.

Only the game thread publishes `READY` after confirming the backend switch. Later ordinary diagnostic events do not overwrite readiness. Bootstrap, installation, and pump failures publish `ERROR`. Status queries and restore also use memory IPC; their results go directly to the caller's console. Windows reclaims communication objects after process exit.

Failure output preserves complete raw errors, error chains, OS codes, managed inner exceptions and stack traces, and timestamped in-memory history. Game exit during initialization includes its process exit code in decimal and hexadecimal. Localized summaries never replace the original error. Collection stops before the complete history is printed to avoid accumulating repeated exceptions while the window waits for Enter.

The tool writes no runtime log or command files, including `launcher-error.log`, `native.log`, `bootstrap.log`, or `payload.log`. Normal game logs, configuration, and saves are unaffected. On failure, close any remaining game instance manually and reopen the tool. On success, the console exits after ten seconds and the game continues running.

## Network, avatars, and localized UI

ENet uses **UDP 30000**. Clients enter the host's IPv4 address without a port in place of an invite code. The host UI still displays `localhost`; other computers must use the host's actual IP. Steam identity initialization remains required; fully offline startup is unsupported.

LAN connections negotiate both game-message and avatar channels. Avatar synchronization uses the game's existing mechanism and still requires each local Steam client to retrieve its player's avatar. The patch copies the Steam texture's original CPU pixels at their source dimensions, bypassing the fixed 128×128 GPU resize. Network avatar copies disable mipmaps without changing global texture quality. Copies have independent lifetimes and use the game's cleanup ownership.

While LAN mode is enabled, the main menu's `VersionAndChangeLog.VersionText` receives one `LAN Enabled` suffix. This does not change the actual game version or multiplayer version checks. Existing menus, newly created menus, language changes, and repeated refreshes are handled without duplicate suffixes. Restore reinstates the current language's original version text.

Localized patches replace all analyzed invite-code text in memory without editing language files:

| UI area | LAN wording or behavior |
| --- | --- |
| Join input and invite-code heading | Host IP |
| Copy button | Copy host IP, preserving keyboard/controller glyphs |
| Generate-invite entry | Use host IP |
| Old and new join descriptions | Obtain the host IPv4 and enter it without a port |
| Main-menu multiplayer tooltip | Join a multiplayer LAN game using the host IP |
| Hosting help | Share the host IPv4, then assign characters after players join |
| Invalid-code error | Invalid host IP |
| Hosting confirmation | Confirm creation of a multiplayer LAN game |

Supported game languages are English, German, French, Italian, Spanish, Brazilian Portuguese, Hungarian, Russian, simplified Chinese, traditional Chinese, Japanese, and Korean.

## Validation and release automation

`cargo xtask test-managed` covers delayed/already-loaded assemblies and timeouts; structural discovery, renamed methods and owner fields, incompatible/ambiguous candidates, constructor activation and rollback; native/managed Unicode readiness and correlated IPC; localized invite-code coverage; version-label refresh, language changes, and restore; ENet channel negotiation; and avatar pixel preservation. Rust tests cover launcher checks, Mono wait behavior, diagnostics, process monitoring, and memory IPC. Automated verification does not start or inject into the game. Real-game startup and multiplayer validation are separate.

Version tags matching `Cargo.toml` trigger Windows CI formatting checks, Rust and managed tests, production ZIP construction and verification, and failure-diagnostic checks before publishing ZIP and SHA256 assets. Release notes repeat the antivirus warnings from both READMEs and include required version-specific changes. Contributor rules and the release procedure are maintained in [AGENTS.md](AGENTS.md).

## Source layout and analysis evidence

| Path | Responsibility |
| --- | --- |
| `src/` | Rust CLI, launcher, Steam discovery, Mono bridge, Windows APIs |
| `managed/` | C# in-process patches, structural discovery, assembly readiness, localized text |
| `xtask/` | Build, run, managed tests, packaging |
| `tools/` | ZIP packaging and regression tools |
| `.cargo/config.toml` | Cargo aliases and linker configuration |
| `.github/workflows/release.yml` | Tag-triggered release workflow |
| `docs/reverse-engineering/` | Historical analysis workflow and evidence |
| `vendor/harmony/` | Harmony DLL, license, and provenance |

[Analysis workflow and evidence](docs/reverse-engineering/README.md) · [Harmony dependency](vendor/harmony/README.md) · [Contributor instructions](AGENTS.md)
