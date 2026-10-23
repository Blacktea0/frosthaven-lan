# Frosthaven LAN：设计与运行说明

[简体中文使用说明](README.md) · [English usage guide](README.en.md) · [English design summary](#english-design-summary)

## 目标与边界

启用 Frosthaven 内置 ENet 后端，复用原有会话、玩家检查与游戏同步流程。补丁只在进程内生效，不替换游戏程序集、Steam DLL 或语言资源。工具不绕过游戏的 Steam 登录或所有权检查。游戏仍会正常写入配置、自动存档及自身日志。

## 架构与注入流程

1. Rust 启动器从 Steam 注册表、Steam 进程路径、`libraryfolders.vdf` 与 `appmanifest_2347080.acf` 发现安装位置，支持多个库与自定义路径。
2. 校验目标进程属于登记安装，并检查游戏程序集哈希；不支持的版本拒绝注入。
3. 向游戏加载原生 `LanBridge.dll`，等待 Mono 根域就绪，再将工作线程附加到运行时。
4. 桥接调用 `Bootstrap.dll`，由它解析运行包内依赖并加载 `LanPayload.dll`。
5. 托管补丁等待 `Assembly-CSharp` 和 `UnityEngine.CoreModule` 加载，再安装 Harmony 补丁。Mono DLL 已出现不代表这些程序集已经就绪；等待用于避免冷启动注入竞争条件。
6. 在游戏主线程上切换后端、刷新文案与输入框，确认 LAN 后端就绪后报告成功。活动联机会话中拒绝切换。

原生桥接等待根域最多 120 秒，托管层等待必需程序集最多 120 秒；启动器注入后的就绪等待为 150 秒。失败或超时通过各阶段日志诊断，不应将 DLL 已加载视为 LAN 已就绪。

## 兼容性校验

当前支持 **Retail 1.0.34121 Steam**。`Assembly-CSharp.dll` SHA256：

```text
20e5c317378426b0b3bf2833e4f54ed34677e062dafcaeecedd1b09340a606c5
```

更换安装目录无需改代码。更换游戏版本需要重新分析、验证接口与补丁位置，再更新支持信息，不能只替换哈希。

## 正式版与开发版

| 行为 | 正式版 | 开发版 |
| --- | --- | --- |
| 构建 | `cargo xtask build` | `cargo xtask build-dev` |
| 运行 | `cargo xtask run` | `cargo xtask dev` |
| 运行包目录 | `dist/` | `dist-dev/` |
| Cargo 输出 | `target/release/` | `target/dev-build/debug/` |
| 权限清单 | `requireAdministrator` | `asInvoker` |
| 无游戏进程时 | 等待手动启动；按 Enter 请求 Steam 启动 | 直接启动登记安装中的游戏 EXE |
| Release 打包 | 包含 | 不包含 |

正式版通过 `steam://rungameid/2347080` 请求 Steam 启动，不直接运行游戏 EXE。未按 Enter 时不会发起请求；请求后 3 分钟仍未检测到游戏会超时。管理员授权被拒绝时，正式版不会启动。

开发版通过 `dev-launcher` feature 编译，单独输出，启动子进程时设置 `SteamAppId`。它不请求提权，也无法访问权限高于自身的游戏进程。游戏自身仍需完成 Steam 身份初始化。

已有游戏进程时复用；多个进程时用 `--pid` 选择。重复运行会查询已加载补丁的实际后端状态，不重复注入。更新补丁或恢复原后端后，需要重启游戏才能重新启用。多实例应使用各自独立的运行包目录，避免共用命令文件。

## 运行包与构建

完整运行包必须包含同目录的五个文件：

```text
frosthaven-lan.exe
LanBridge.dll
Bootstrap.dll
LanPayload.dll
0Harmony.dll
```

Rust 负责启动、注入、Mono 桥接与任务管理；C# 通过运行时反射解析游戏类型，因此编译无需本机游戏程序集。Windows x64 MSVC 为正式发布目标，原生 C 运行库静态链接。构建前需要退出游戏，避免覆盖已加载的 DLL。

`cargo xtask package` 强制使用不启用开发 feature 的 release 构建，只从 `dist/` 按明确文件列表生成 ZIP，附带双语 README、`PRODUCT.md` 与 Harmony 许可证，不包含运行日志或开发版。产物为：

```text
artifacts/frosthaven-lan-v<版本>-windows-x64.zip
artifacts/frosthaven-lan-v<版本>-windows-x64.zip.sha256
```

打包会解压核对文件列表与内容哈希，并从 EXE 资源中读取清单，确认 `requireAdministrator` 和 `uiAccess=false`。最终用户无需安装 Rust。

## 高级命令与恢复

以下正式版命令在管理员终端运行：

```text
.\dist\frosthaven-lan.exe start --pid <PID>
.\dist\frosthaven-lan.exe inject <PID>
cargo xtask locate
cargo xtask verify
cargo xtask command status
cargo xtask command steam-status
cargo xtask command restore
```

- `start` 等待运行时、注入并确认后端就绪；`inject` 仅执行注入步骤，不等待 LAN 就绪。
- `locate` 显示 Steam 中的安装目录；`verify` 校验游戏程序集。
- `status` 查询当前后端与会话状态；`steam-status` 查询 Steam 身份状态。
- `restore` 需先结束多人会话，然后恢复原网络后端、撤销方法补丁并刷新界面。DLL 和输入框实例配置在游戏退出后完全释放。

开发版也支持相同的 EXE 子命令，将路径换为 `.\dist-dev\frosthaven-lan.exe` 即可。

## 日志与故障排查

应用输出及日志使用英文。日志位于当前注入所使用的运行包目录；游戏日志沿用游戏自身默认位置。

| 文件 | 内容 |
| --- | --- |
| `launcher-error.log` | 无参数启动时记录的启动器失败信息 |
| `native.log` | 原生桥接、Mono 根域及 Bootstrap 调用诊断 |
| `bootstrap.log` | 托管依赖解析或入口调用失败 |
| `payload.log` | 程序集等待、Harmony 安装、后端状态及命令执行结果 |

- `WAITING for runtime assemblies` 表示运行时仍在加载，等待后续结果；超时会列出未加载程序集。
- `INSTALL ERROR`、`PUMP ERROR` 或 `REFUSED:` 表示补丁安装、主线程操作或会话状态检查失败。
- 只有 `LAN backend ready` 才表示启动器已经确认后端就绪。
- 注入失败时先保留上述日志；重试前重启游戏，以免复用部分安装或已恢复的状态。

## 网络与文案

ENet 使用 **UDP 30000**。客户端以房主 IPv4 代替原邀请码输入；不附加端口。房主界面目前仍显示 `localhost`，其他电脑须使用房主实际 IP。

补丁随游戏当前语言替换以下词条，语言文件保持原样：

| 词条 | 简体中文 |
| --- | --- |
| `GUI_MULTIPLAYER_ENTER_INVITE_CODE` | 输入主机IP |
| `GUI_MULTIPLAYER_HOSTGAME_CONFIRMATION_DESC` | 你想要建立多人局域网游戏吗？ |

支持英语、德语、法语、意大利语、西班牙语、巴西葡萄牙语、匈牙利语、俄语、简体中文、繁体中文、日语和韩语。

## 验证与自动发布

托管回归测试 `cargo xtask test-managed` 覆盖程序集延迟加载、已加载和超时路径，不启动或注入游戏。Rust 正式与开发 feature 配置分别通过 Cargo 测试命令检查；实机注入测试与多人联机测试需要单独执行。

推送与 `Cargo.toml` 版本一致的 `v*` tag，会触发 Windows GitHub Actions，执行格式检查、两种 feature 配置测试、托管回归测试、正式 ZIP 构建及完整性校验，然后发布附带 ZIP 与 SHA256 的 GitHub Release。Release 描述重复双语 README 开头的误报警告。

## 源码目录与分析材料

| 路径 | 职责 |
| --- | --- |
| `src/` | Rust CLI、启动器、Steam 发现、Mono 桥接与 Windows API |
| `managed/` | C# 进程内补丁、运行时程序集等待与多语言文案 |
| `xtask/` | 编译、运行、托管测试与打包入口 |
| `tools/` | ZIP 打包、测试与只读分析工具 |
| `.cargo/config.toml` | Cargo 命令别名与链接配置 |
| `.github/workflows/release.yml` | tag 自动发布流程 |
| `docs/reverse-engineering/` | 逆向分析流程与原始证据 |
| `vendor/harmony/` | Harmony DLL、许可证与来源 |

[逆向流程与证据](docs/reverse-engineering/README.md) · [Harmony 依赖说明](vendor/harmony/README.md) · [贡献约定](AGENTS.md)

## English design summary

The Rust launcher discovers Steam installations and verifies both the process path and the supported game assembly hash. It injects `LanBridge.dll`, waits for the Mono root domain, and invokes `Bootstrap.dll`. The bootstrap resolves packaged dependencies and loads the managed payload. The payload waits for `Assembly-CSharp` and `UnityEngine.CoreModule` before installing Harmony patches, then switches the backend and refreshes UI on the game thread. An active network session prevents switching.

Production embeds `requireAdministrator` and requests startup through the Steam protocol only when Enter is pressed. A Steam startup request times out after three minutes. Dev uses `asInvoker`, separate build/runtime directories, and direct game EXE startup with `SteamAppId`; game-side Steam authentication still applies. Existing processes are reused, `--pid` selects between instances, and repeated launches query the actual backend. Restart the game after payload updates or restore.

`cargo xtask package` always builds production with dev features disabled. It archives the explicit runtime allowlist from `dist/`, both READMEs, `PRODUCT.md`, and the Harmony license. It verifies extracted files, hashes, and the embedded administrator manifest, then writes the ZIP and SHA256 to `artifacts/`. Dev binaries and logs are excluded. Building uses runtime reflection and does not require installed game assemblies.

Advanced commands include `start --pid <PID>`, `inject <PID>`, `locate`, `verify`, and `command <status|steam-status|restore>`. `start` confirms backend readiness; `inject` only loads the bridge. End multiplayer before restore. Native diagnostics are in `native.log`, dependency/entry failures in `bootstrap.log`, and patch/backend/command output in `payload.log`. No-argument launcher failures are saved in `launcher-error.log`. A loaded DLL alone does not confirm a ready LAN backend.

ENet uses UDP 30000 and the client's invite-code field becomes a host-IPv4 field. Other computers must use the actual host IP rather than the host UI's `localhost`. Game text follows all 12 supported languages without modifying language files. Patches affect memory only; normal game configuration and save writes continue.

Version tags matching `Cargo.toml` trigger Windows CI formatting checks, both feature configurations, managed readiness regression tests, production packaging, and Release publication. Both README warnings are repeated in release notes. Live injection and multiplayer testing remain separate from automated build verification. See the tables and commands above for the supported hash, runtime files, log locations, and source layout.
