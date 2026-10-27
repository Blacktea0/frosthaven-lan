# Frosthaven LAN：设计与运行说明

[简体中文使用说明](README.md) · [English usage guide](README.en.md) · [English design summary](#english-design-summary)

## 目标与边界

启用 Frosthaven 内置 ENet 后端，复用原有会话、玩家检查与游戏同步流程。补丁只在进程内生效，不替换游戏程序集、Steam DLL 或语言资源。工具不绕过游戏的 Steam 登录或所有权检查。游戏仍会正常写入配置、自动存档及自身日志。

## 架构与注入流程

1. Rust 启动器从 Steam 注册表、Steam 进程路径、`libraryfolders.vdf` 与 `appmanifest_2347080.acf` 发现安装位置，支持多个库与自定义路径。
2. 校验目标进程属于登记安装，记录游戏程序集 SHA256，并在启动及注入前检查文件未发生变化；不使用固定哈希白名单。
3. 向游戏加载原生 `LanBridge.dll`，等待 Mono 根域和托管主线程对象出现，再将工作线程附加到运行时。
4. 桥接调用 `Bootstrap.dll`，由它解析运行包内依赖并加载 `LanPayload.dll`。
5. 托管补丁等待 `Assembly-CSharp` 和 `UnityEngine.CoreModule` 加载，再安装 Harmony 补丁。Mono DLL 已出现不代表这些程序集已经就绪；等待用于避免冷启动注入竞争条件。
6. 在游戏主线程上切换后端、刷新文案与输入框，确认 LAN 后端就绪后报告成功。活动联机会话中拒绝切换。

原生桥接等待根域和主线程对象最多 120 秒，托管层等待必需程序集最多 120 秒；启动器注入后的就绪等待为 150 秒。失败或超时通过内存状态通信反馈到控制台，不应将 DLL 已加载视为 LAN 已就绪。

## 兼容性校验

原支持基线为 **Retail 1.0.34121 Steam**，其 `Assembly-CSharp.dll` SHA256（仅作历史指纹）：

```text
20e5c317378426b0b3bf2833e4f54ed34677e062dafcaeecedd1b09340a606c5
```

更新版本通过运行时结构探测兼容：在 `FFSNetwork` 及其嵌套类型内寻找唯一的 EOS 构造调用，并要求下一条有效指令写入静态 `_backend` 字段；ENet 必须提供参数类型完全一致的构造函数。ENet 连接点按 `ENet.Host.Connect(Address)` 调用定位，头像协程按 GPU 复制流程和唯一的玩家引用字段定位。所有候选必须唯一，关键签名和原有 IL 防护必须通过；安装主线程 Pump 前完成其他补丁，失败时撤销本工具的补丁。

不依赖元数据 token、协程编号或上述方法名称，但后端类型名、网络属性和界面接口仍必须兼容。不扫描任意程序集来猜测替代接口，不自动放宽协议或头像复制的结构要求。SHA256 只用于诊断和防止启动期间文件变化，不能证明新版本的协议或联机行为兼容。

2026-10-28 对更新后程序集进行了只读元数据/IL 检查，SHA256 为 `c16c02d74985fafdf5c02ab0c5d33ca5c5cf646be4b064474f8f40989e4ff13c`；后端构造与字段赋值、ENet 连接、头像复制和现有 UI 入口结构仍符合探测条件。未启动或注入游戏，未验证该版本的实际启动或多人联机。

## 按权限选择启动方式

只维护一个构建版本，运行包输出到 `dist/`，Cargo 输出到 `target/release/`。EXE 使用 `asInvoker` 清单，保留调用者权限，不自动请求提权。启动时通过进程令牌的 `TokenElevation` 检查实际权限，避免将管理员账户的普通 UAC 登录态误判为已提权。

| 当前启动权限 | 无游戏进程时的行为 | 使用方法 |
| --- | --- | --- |
| 普通权限 | 直接启动登记安装中的 `Frosthaven.exe`，设置 `SteamAppId` | 双击启动器，或在普通终端运行 |
| 管理员权限 | 立即通过 `steam://rungameid/2347080` 请求 Steam 启动游戏 | 右键“以管理员身份运行”，或使用管理员终端 |

如需启用 Steam 游戏内叠加界面，请使用管理员模式启动工具。如需通过 Steam 启动，请先关闭游戏，再以管理员身份运行工具，由工具发起启动；普通权限无法访问较高权限的游戏进程。直接启动仍需游戏完成 Steam 身份初始化，不绕过登录或所有权检查。

Steam 启动请求后 3 分钟仍未检测到游戏会超时。两种模式都会自动开始启动，无需按 Enter；成功后等待十秒自动退出，Enter 用于失败后关闭窗口，命令行入口同样会等待。README 说明 Steam 叠加界面的管理员启动方式，正常控制台只显示五个启动阶段。

工具启动时以及发起游戏启动前都会检查是否存在游戏进程；如果已经运行，提示用户手动关闭所有实例后重新打开工具，不注入、不结束游戏，也不接管已有进程。已移除 `--pid` 和直接注入子命令。普通模式只注入本次创建的子进程；Steam 模式只在本次启动请求后等待新游戏进程，多实例歧义时拒绝继续。更新补丁或恢复原后端后，需要手动关闭游戏，再重新打开工具才能启用。

## 运行包与构建

完整运行包必须包含同目录的五个文件：

```text
frosthaven-lan.exe
LanBridge.dll
Bootstrap.dll
LanPayload.dll
0Harmony.dll
```

Rust 负责启动、注入、Mono 桥接与任务管理；C# 通过运行时反射解析游戏类型，因此编译无需本机游戏程序集。GitHub Actions 使用 Windows x64 MSVC 发布构建；本地支持 MSVC 或 GNU，沿用本机配置的默认工具链，原生 C 运行库静态链接。MSVC 必须安装 Windows SDK 链接库（包括 `kernel32.lib`），GNU 需要 GCC 与 `windres`。构建前需要退出游戏，避免覆盖已加载的 DLL。

`cargo xtask package` 使用统一的 release 构建，只从 `dist/` 按明确文件列表生成 ZIP，附带双语 README、`PRODUCT.md` 与 Harmony 许可证，不包含运行状态文件。产物为：

```text
artifacts/frosthaven-lan-v<版本>-windows-x64.zip
artifacts/frosthaven-lan-v<版本>-windows-x64.zip.sha256
```

ZIP 内所有文件统一放在 `frosthaven-lan/` 文件夹中，顶层不散放文件。ZIP 文件名保留版本号，此结构从下一次打包发布开始使用。

打包会解压确认顶层仅有该文件夹，再核对其中的文件列表与内容哈希，并从 EXE 资源中读取清单，确认 `asInvoker` 和 `uiAccess=false`。最终用户无需安装 Rust。

## 高级命令与恢复

以下命令按当前进程权限运行；操作 Steam 启动的游戏时应使用管理员终端：

```text
.\dist\frosthaven-lan.exe start
cargo xtask locate
cargo xtask verify
cargo xtask command status
cargo xtask command steam-status
cargo xtask command restore
```

- `start` 检查游戏已关闭，按当前权限启动游戏、等待运行时、注入并确认后端就绪。
- `locate` 显示 Steam 中的安装目录；`verify` 输出游戏程序集指纹，补丁结构兼容性在启动后检查。
- `status` 查询当前后端与会话状态；`steam-status` 查询 Steam 身份状态。
- `restore` 需先结束多人会话，然后恢复原网络后端、撤销方法补丁并刷新界面。DLL 和输入框实例配置在游戏退出后完全释放。

## 控制台、locale 与内存通信

控制台根据 Windows 用户 locale 选择语言：`zh` 系列 locale 显示简体中文，其他 locale 显示英文，无法检测 locale 时默认简体中文。进度等待时显示动态转圈；输出重定向到文件或管道时，每个阶段仅输出一行静态文本，不输出动画控制字符。

正常启动按顺序显示以下五步（进行中的省略号由动态转圈替代）：

```text
管理员模式/非管理员模式启动中
定位到游戏目录：（游戏实际安装目录）
正在启动游戏中
检测到游戏进程 (PID:123456)，正在等待网络组件初始化
网络组件注入成功，强制局域网联机模式已开启，十秒后将关闭该窗口
```

为避免控制槽位长度限制截断异常，完整诊断通过独立命名管道 `FrosthavenLan-Diagnostics-<PID>` 传输，按 UTF-8 字节长度分帧，接收端保存完整记录后才回执。管道仅允许同一 Windows 用户与 SYSTEM 连接，使用 medium 完整性标签以支持普通权限游戏向同用户管理员启动器报告错误。正常成功路径不显示技术诊断；失败时停止收集并输出完整历史，避免停留窗口期间反复累积同一游戏异常。

不生成 `launcher-error.log`、`native.log`、`bootstrap.log`、`payload.log` 或命令文件。原生桥接在游戏进程中建立 `Local\FrosthavenLan-<PID>` 内存映射，Bootstrap 和补丁共享该映射。独立槽位传递启动状态、命令请求和命令结果，使用序列号防止读取未完成的写入。命令请求携带唯一标识，结果匹配请求标识后显示在调用方控制台。

只有主线程完成后端切换才发布 `READY`；后续普通诊断事件不会覆盖就绪状态。安装、Bootstrap 或主线程失败发布 `ERROR`，启动器直接显示错误。`status`、`steam-status` 和 `restore` 也通过内存通信，结果仅输出到控制台。游戏退出后由系统回收通信对象。游戏自身的正常日志、配置和存档写入不受影响。

失败时完整显示原始错误及其错误链、托管异常的内层异常与调用栈、Windows 错误码和分阶段诊断记录；本地化摘要不会替代原始错误。诊断记录保存在内存中，仅在失败时输出。手动关闭游戏后重新打开工具。所有启动入口失败后等待 Enter，成功后显示转圈等待十秒再退出，游戏继续运行。

## 网络与文案

ENet 使用 **UDP 30000**。客户端以房主 IPv4 代替原邀请码输入；不附加端口。房主界面目前仍显示 `localhost`，其他电脑须使用房主实际 IP。

LAN 后端启用后，在主菜单 `VersionAndChangeLog.VersionText` 的原有版本号后追加 `LAN Enabled`。只修改界面显示，不改变实际游戏版本或联机版本校验。覆盖已经存在的主菜单、新建菜单及语言切换后的刷新，重复刷新不会叠加标记；恢复补丁时恢复当前语言对应的原始版本文本。

补丁随游戏当前语言覆盖所有已分析的邀请码文案，包括以下入口，语言文件保持原样：

| 界面 | 简体中文文案 |
| --- | --- |
| 加入输入框 | 输入主机IP |
| 邀请码字段标题 | 主机IP |
| 复制按钮 | 复制主机IP（保留键盘/手柄提示图标） |
| 生成邀请入口 | 使用主机IP |
| 新旧加入界面说明 | 获取主机 IPv4，输入时不附加端口 |
| 主菜单多人提示 | 使用主机IP加入多人局域网游戏 |
| 开房帮助 | 创建会话后分享主机 IPv4，玩家加入后分配角色 |
| 无效邀请码错误 | 主机IP无效 |
| 开房确认 | 你想要建立多人局域网游戏吗？ |

支持英语、德语、法语、意大利语、西班牙语、巴西葡萄牙语、匈牙利语、俄语、简体中文、繁体中文、日语和韩语。

## 验证与自动发布

托管回归测试 `cargo xtask test-managed` 覆盖程序集延迟加载、已加载和超时路径，原生与托管进程之间的 Unicode 状态、请求关联和响应传输，以及全部邀请码词条覆盖、版本标记刷新/语言切换/恢复，不启动或注入游戏。Rust 代码通过 Cargo 测试命令检查；实机注入测试与多人联机测试需要单独执行。

推送与 `Cargo.toml` 版本一致的 `v*` tag，会触发 Windows GitHub Actions，执行格式检查、Rust 测试、托管回归测试、正式 ZIP 构建及完整性校验，然后发布附带 ZIP 与 SHA256 的 GitHub Release。Release 描述重复双语 README 开头的误报警告。

## 源码目录与分析材料

| 路径 | 职责 |
| --- | --- |
| `src/` | Rust CLI、启动器、Steam 发现、Mono 桥接与 Windows API |
| `managed/` | C# 进程内补丁、运行时程序集等待与多语言文案 |
| `xtask/` | 编译、运行、托管测试与打包入口 |
| `tools/` | ZIP 打包与回归测试工具 |
| `.cargo/config.toml` | Cargo 命令别名与链接配置 |
| `.github/workflows/release.yml` | tag 自动发布流程 |
| `docs/reverse-engineering/` | 逆向分析流程与原始证据 |
| `vendor/harmony/` | Harmony DLL、许可证与来源 |

[逆向流程与证据](docs/reverse-engineering/README.md) · [Harmony 依赖说明](vendor/harmony/README.md) · [贡献约定](AGENTS.md)

## English design summary

The Rust launcher discovers Steam installations, verifies the process path, and records the assembly fingerprint, checking that it remains unchanged during startup rather than enforcing a fixed hash allowlist. It injects `LanBridge.dll`, waits for both the Mono root domain and managed main thread object, and invokes `Bootstrap.dll`. The payload waits for the game and Unity assemblies, then discovers unique backend initialization, ENet connection and avatar copy methods from their IL structure. Exact constructor signatures, backend properties and existing IL guards remain mandatory; incompatible or ambiguous structures stop installation. Other patches are installed before the game-thread pump, with rollback on failure. Backend type names and UI interfaces must still be compatible; structural checks cannot prove network protocol compatibility. An active network session prevents switching.

On 2026-10-28, read-only metadata/IL inspection of the updated assembly (`c16c02d74985fafdf5c02ab0c5d33ca5c5cf646be4b064474f8f40989e4ff13c`) found compatible backend construction/assignment, ENet connection, avatar copy and existing UI entry structures. This was not a real-game startup, injection or multiplayer test. The old hash above is a historical baseline, not an allowlist entry. The `verify` command prints a fingerprint; patch compatibility is checked during startup.

One `asInvoker` launcher preserves caller permissions and queries the effective process token for elevation. Standard permissions launch the game EXE directly with `SteamAppId`; administrator permissions request startup through Steam immediately. Neither path waits for Enter. Use administrator mode if you need the Steam in-game overlay. Close the game first, then right-click the launcher and select Run as administrator to let the tool launch it through Steam. Game-side Steam authentication still applies. A Steam startup request times out after three minutes. Existing game processes are rejected: close all instances manually, then reopen the tool. PID selection and direct injection commands have been removed. Standard mode targets only its newly created child; Steam mode waits for a new process after its own startup request and rejects multiple instances. After updates or restore, close the game and reopen the tool.

`cargo xtask package` builds the single release runtime. It archives the explicit runtime allowlist from `dist/`, both READMEs, `PRODUCT.md`, and the Harmony license. Starting with the next packaged release, all files are nested under a single `frosthaven-lan/` directory; the ZIP filename retains the version number. It verifies that top-level layout, extracted files, hashes, and the embedded `asInvoker` manifest, then writes the ZIP and SHA256 to `artifacts/`. Runtime state files are excluded. Building uses runtime reflection and does not require installed game assemblies.

Advanced commands include `start`, `locate`, `verify`, and `command <status|steam-status|restore>`. `start` requires the game to be closed, launches it, and confirms backend readiness. End multiplayer before restore. Runtime errors and command results go directly to the console through a process-lifetime named memory mapping; no log or command files are written. A loaded DLL alone does not confirm a ready LAN backend.

ENet uses UDP 30000 and the client's invite-code field becomes a host-IPv4 field. Other computers must use the actual host IP rather than the host UI's `localhost`. All analyzed invite-code labels, copy/use buttons, join descriptions, tooltips, hosting help, and invalid-code messages use host-IP wording in all 12 supported languages without modifying language files. The main-menu version label gains a single `LAN Enabled` suffix only while the LAN patch is enabled; existing menus, newly created menus, language changes, and restore are handled without changing the real game version. Patches affect memory only; normal game configuration and save writes continue.

Version tags matching `Cargo.toml` trigger Windows CI formatting checks, Rust tests, managed readiness regression tests, production packaging, and Release publication. Both README warnings are repeated in release notes. Live injection and multiplayer testing remain separate from automated build verification. See the sections above for compatibility checks, runtime files, console behavior, and source layout.

Console language follows the Windows user locale: Chinese locales use simplified Chinese, other locales fall back to English, and unavailable locale detection defaults to simplified Chinese. The five startup stages use animated spinners in a terminal and one static line each when redirected. Successful startup waits ten seconds and exits; every invocation waits for Enter after failure. Startup status and correlated command responses travel through shared memory, not files. The game continues running after launcher exit.

Failure diagnostics retain raw errors, error chains, OS error codes, managed inner exceptions, and stack traces in a timestamped in-memory history. Full diagnostic records use a length-framed, acknowledged named pipe instead of the limited shared-memory status slot, so large Unicode traces are not truncated. The same Windows user and SYSTEM can access the pipe, including communication from a standard game to an elevated launcher. All launch modes wait for Enter after a failure; only success uses the ten-second automatic exit. No runtime log files are written.
