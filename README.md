> [!CAUTION]
> **杀毒软件误报警告**
> 本工具需要向游戏进程注入 DLL 并在内存中打补丁，因此可能被杀毒软件误判。官方 Release 二进制包由本仓库的 GitHub Actions 从对应 tag 的源码构建；这不代表绝对安全。如果担心病毒，请自行阅读源码，或让 AI 辅助审查，然后在自己的机器上编译。如果仍然不放心，请不要使用。不要为了使用本工具而关闭杀毒软件。

# Frosthaven LAN

[English](README.en.md)

## 简介

Windows x64 的 Frosthaven 局域网联机工具，通过内存注入启用游戏内置的 ENet 后端，不修改游戏安装文件。当前支持 **Retail 1.0.34121 Steam**。

启用后主菜单版本号末尾显示 `LAN Enabled`，联机界面中的邀请码提示改为使用主机 IP。

LAN 连接启用游戏消息和头像两个通道，支持沿用游戏原有机制同步玩家 Steam 头像；头像仍需由各玩家本机 Steam 成功取得。

头像从 Steam 纹理的原始像素生成可读副本，保留源尺寸，避免原游戏的固定 128×128 GPU 缩放；网络头像禁用 mipmap，不改变游戏整体画质设置。`command status` 同时输出当前头像尺寸、mipmap 状态和 PNG SHA256，便于检查同步一致性。

详细设计、命令和故障排查见 [PRODUCT.md](PRODUCT.md)。

## 如何使用

1. 安装并登录 Steam，确认游戏可以正常启动。双方使用相同的游戏版本和补丁。
2. 从 [Releases](https://github.com/Blacktea0/frosthaven-lan/releases/latest) 下载 ZIP，**完整解压**到可写目录，不要只复制 EXE。
3. 选择启动方式：**普通双击** `frosthaven-lan.exe`，工具会直接启动游戏本体；**需要 Steam 游戏内叠加界面时，右键 → 以管理员身份运行**，工具会通过 Steam 启动游戏。**必须由本工具启动游戏**；如果检测到游戏已运行，请手动关闭所有游戏实例，再重新打开本工具。
4. 等待提示“网络组件注入成功，强制局域网联机模式已开启”。房主载入战役，从暂停菜单进入“多人游戏”开房。
5. 客户端选择“多人游戏 → 加入游戏”，输入房主 IPv4（例如 `192.168.1.20`，不带端口）；房主分配角色并确认。

如需开启 **Steam 游戏内叠加界面**，请使用管理员模式启动本工具。普通模式直接启动游戏本体；两种模式均由工具启动游戏。

网络需允许 **UDP 30000**。房主界面显示的 `localhost` 不能用于其他电脑连接，请使用房主实际 IP。仍需 Steam 身份初始化，不支持完全离线启动。

启动步骤以动态转圈显示进度。控制台根据 Windows locale 显示简体中文或英文：中文环境使用简体中文，其他语言环境使用英文，无法检测语言时默认简体中文。成功后等待十秒自动退出，游戏继续运行；失败时控制台会展示原始完整错误、内层异常、调用栈、Windows 错误码和带时间戳的诊断记录，所有启动方式均等待按 Enter 关闭，不会自动倒计时退出。工具不生成日志文件。更新补丁后请关闭游戏，再重新打开工具。

## 开发环境需求

- Windows x64。
- Rust stable，沿用本机默认工具链，支持以下任一组合：
  - MSVC：`x86_64-pc-windows-msvc`，配套 Visual Studio C++ Build Tools 和 Windows SDK（包含 `kernel32.lib`）。
  - GNU：`x86_64-pc-windows-gnu`，配套 MinGW-w64/GCC，确保 `gcc` 和 `windres` 在 PATH 中。
- Windows .NET Framework C# 编译器：`C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe`。
- 系统 PowerShell，用于 ZIP 打包。

Harmony 2.3.3 已包含在仓库中。编译无需安装游戏；实际注入测试需要支持版本的游戏及 Steam 环境。

## 如何编译开发

在项目根目录执行，重建前先退出游戏。

```text
cargo xtask build           # 构建正式版到 dist/
cargo xtask package         # 生成正式版 ZIP 和 SHA256 到 artifacts/
cargo xtask run             # 构建并运行，按当前终端权限选择启动方式
```

只有一个运行版本：普通终端直接启动游戏，管理员终端通过 Steam 启动。请使用 `xtask` 生成完整运行包，`cargo build` 仅构建 Rust 部分。

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo xtask test-managed
```
