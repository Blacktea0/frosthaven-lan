> **杀毒软件误报警告**
> 本工具需要向游戏进程注入 DLL 并在内存中打补丁，因此可能被杀毒软件误判。官方 Release 二进制包由本仓库的 GitHub Actions 从对应 tag 的源码构建；这不代表绝对安全。如果担心病毒，请自行阅读源码，或让 AI 辅助审查，然后在自己的机器上编译。如果仍然不放心，请不要使用。不要为了使用本工具而关闭杀毒软件。

# Frosthaven LAN

[English](README.en.md)

Windows x64 的 Frosthaven 局域网联机注入工具。将游戏网络后端切换为内置 ENet，保留原有会话、玩家检查和游戏同步流程。所有补丁仅在进程内生效，不修改或替换游戏安装目录中的程序、程序集、Steam DLL 或语言资源。

Rust 管理构建、游戏启动、注入和原生 Mono 桥接；`managed/` 中的 C# 实现 Harmony 后端与文案补丁。运行无需 Python；ZIP 打包使用系统 PowerShell。

## 环境

- Windows x64，已安装并登录 Steam，能够正常启动 Frosthaven。
- Rust stable x64 MSVC 工具链及 Visual Studio C++ Build Tools。
- Windows .NET Framework C# 编译器：`C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe`。
- Harmony 2.3.3 DLL 和许可已保留在 `vendor/harmony/`。

当前支持 Retail 1.0.34121 Steam。工具自动读取 Steam 注册表信息、Steam 进程路径、`libraryfolders.vdf` 和游戏清单 `appmanifest_2347080.acf`，定位实际安装目录，支持多个 Steam 库及自定义库位置。

`Assembly-CSharp.dll` SHA256：

```text
20e5c317378426b0b3bf2833e4f54ed34677e062dafcaeecedd1b09340a606c5
```

注入器核对进程是否来自 Steam 登记的游戏安装，并检查程序集哈希；版本不符会拒绝注入。更换安装目录无需改代码；更换游戏版本需重新分析后再更新支持信息。

## 双击使用

解压完整 ZIP 后，直接双击 `frosthaven-lan.exe` 即可，无需输入参数：

- 游戏未启动：显示提示并持续等待。你可以自行前往 Steam 启动游戏，也可以在工具窗口按 Enter，由工具通过 `steam://rungameid/2347080` 请求 Steam 启动。未按 Enter 时不会发起启动请求。
- 游戏已启动：使用已有进程进行注入，不再启动第二个实例。
- 游戏已启用补丁：查询实际后端状态，提示已启用，不重复注入。

游戏开始运行后，工具会自动检测进程并完成注入。工具不直接运行游戏 EXE；按 Enter 帮助启动时仅调用已注册的 Steam 协议。若等待 Steam 启动超过 3 分钟仍未检测到游戏，会显示超时提示。

启动器内嵌 `requireAdministrator` 清单，启动时必须通过 Windows UAC 管理员授权；拒绝授权则不会启动。命令行使用时请先打开管理员终端。工具复用已经由 Steam 启动的游戏进程。

双击打开的控制台会保留结果，注入结束后按 Enter 关闭工具窗口，游戏继续运行。失败信息同时写入运行包中的 `launcher-error.log`。必须将完整运行包的五个文件放在同一目录，不要只复制 EXE。

## 构建与启动

在项目根目录执行：

```text
cargo xtask build
cargo xtask package
cargo xtask run
```

`build` 调用 Cargo 和 C# 编译器，把完整运行包放到 `dist/`。构建无需安装游戏，游戏类型在运行时通过反射解析。Rust 原生代码静态链接 C 运行库，目标为 `x86_64-pc-windows-msvc`。

`package` 先构建，再将运行文件、双语说明及 Harmony 许可证打包为 `artifacts/frosthaven-lan-v<版本>-windows-x64.zip`，同时生成 SHA256 校验文件。打包会解压验证文件列表、内容哈希和管理员清单，避免把运行日志或旧文件打进包里。解压全部文件到可写目录后运行 EXE，无需安装 Rust。

构建前需退出游戏，以免覆盖已加载的 DLL。

`run` 在没有游戏进程时自动构建，然后显示启动提示，等待你从 Steam 启动游戏，或按 Enter 请求 Steam 启动；检测到进程后等待运行时初始化，注入并确认 LAN 后端就绪。若已有一个尚未注入的游戏进程，则使用现有运行包进行注入。多个进程时指定 PID：

```text
cargo xtask run --pid <PID>
```

重复打开启动器会查询已加载补丁的状态；更新补丁或执行 restore 后需重启游戏才能重新启用。多个安装或多个运行实例存在歧义时，请先启动目标游戏并通过 `--pid` 选择。

## 开发版与注入测试

```text
cargo xtask build-dev
cargo xtask dev
cargo xtask dev --pid <PID>
cargo xtask test-managed
```

开发版单独构建到 `target/dev-build/`，完整运行文件放在 `dist-dev/`。它使用 `asInvoker` 清单，不请求管理员权限；没有现成游戏进程时直接启动 Steam 登记目录中的 `Frosthaven.exe`，无需通过 Steam 启动。Steam 身份初始化仍由游戏处理，开发版不会绕过登录或所有权检查；它也无法访问权限高于自身的游戏进程。已有进程时通过 `--pid` 选择。

修复了冷启动注入时 Mono 已加载、但游戏/Unity 托管程序集尚未就绪的竞争条件。桥接会等待 Mono 根域，托管补丁会等待必需程序集加载，超时和 Bootstrap 失败会提供对应诊断。`test-managed` 测试延迟加载、已就绪和超时分支，不会启动游戏。

`cargo xtask build` 和 `cargo xtask package` 始终构建正式版，不启用开发 feature，仍要求管理员权限并通过 Steam 请求启动。打包只读取 `dist/` 并验证 `requireAdministrator`，不会把 `dist-dev/` 放进 Release。

## 联机

1. 双方使用相同游戏版本和补丁，并保证网络可达。
2. 房主载入战役，在暂停菜单中选择“多人游戏”开房。
3. 客户端选择“多人游戏 → 加入游戏”，在“输入主机IP”框输入房主 IPv4，例如 `192.168.1.20`，不附加端口。
4. 房主为加入的玩家分配角色，再确认进入游戏。

游戏使用 **UDP 30000**。房主界面目前仍显示 `localhost`，另一台电脑应使用房主实际 IP。Steam 身份初始化仍保留，不支持完全离线启动。

多实例应各用独立的运行包目录，避免抢读同一个命令文件。

## 独立 EXE

完整运行包包含以下五个文件，必须放在同一目录：

```text
frosthaven-lan.exe
LanBridge.dll
Bootstrap.dll
LanPayload.dll
0Harmony.dll
```

构建完成后，无需安装 Rust 即可使用：

```text
.\dist\frosthaven-lan.exe
.\dist\frosthaven-lan.exe start --pid <PID>
.\dist\frosthaven-lan.exe inject <PID>
```

`cargo build --release --locked` 仅构建 Rust 部分；生成完整运行包请使用 `cargo xtask build`。

## 状态与恢复

```text
cargo xtask locate
cargo xtask verify
cargo xtask command status
cargo xtask command steam-status
cargo xtask command restore
```

`locate` 显示 Steam 中找到的游戏目录，`verify` 只校验游戏程序集。其余命令由已注入进程的主线程处理，结果写入 `dist/payload.log`；桥接日志在 `dist/native.log`。游戏由 Steam 启动，沿用游戏自身的默认日志位置，不再设置自定义启动参数。

`restore` 要求先结束多人会话，然后恢复原网络后端、撤销方法补丁并刷新界面。DLL 与输入框实例配置会在游戏退出后完全释放。游戏本身仍会按正常行为写入配置与自动存档。

## 多语言文案

覆盖全部 12 种游戏语言，并随当前语言动态返回对应文本：

| 词条 | 简体中文 |
| --- | --- |
| `GUI_MULTIPLAYER_ENTER_INVITE_CODE` | 输入主机IP |
| `GUI_MULTIPLAYER_HOSTGAME_CONFIRMATION_DESC` | 你想要建立多人局域网游戏吗？ |

语言包括英语、德语、法语、意大利语、西班牙语、巴西葡萄牙语、匈牙利语、俄语、简体中文、繁体中文、日语和韩语。语言源文件不会被修改。

## 目录

```text
src/                      Rust CLI、启动器、Steam 目录发现、桥接和 Windows API
managed/                  C# 进程内补丁
xtask/                    Rust 构建与任务管理
.cargo/config.toml        Cargo 命令别名
docs/reverse-engineering/ 逆向流程和原始分析材料
tools/                    只读分析辅助源码
vendor/harmony/           Harmony DLL、许可及来源
target/                   Cargo 生成目录（忽略）
dist/                     完整运行包（忽略）
logs/                     运行日志（忽略）
```

[逆向流程与证据](docs/reverse-engineering/README.md) · [Harmony 依赖说明](vendor/harmony/README.md)






## 自动发布

推送与 `Cargo.toml` 版本一致的 `v*` tag（例如 `v0.2.0`）会触发 GitHub Actions，在 Windows runner 上构建、校验 ZIP，并发布带 ZIP 和 SHA256 校验文件的 GitHub Release。Release 描述包含相同的误报警告。可在 Actions 中查看构建日志。
