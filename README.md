> **杀毒软件误报警告**
> 本工具需要向游戏进程注入 DLL 并在内存中打补丁，因此可能被杀毒软件误判。官方 Release 二进制包由本仓库的 GitHub Actions 从对应 tag 的源码构建；这不代表绝对安全。如果担心病毒，请自行阅读源码，或让 AI 辅助审查，然后在自己的机器上编译。如果仍然不放心，请不要使用。不要为了使用本工具而关闭杀毒软件。

# Frosthaven LAN

[English](README.en.md)

## 简介

Windows x64 的 Frosthaven 局域网联机工具，通过内存注入启用游戏内置的 ENet 后端，不修改游戏安装文件。当前支持 **Retail 1.0.34121 Steam**。

详细设计、命令和故障排查见 [PRODUCT.md](PRODUCT.md)。

## 如何使用

1. 安装并登录 Steam，确认游戏可以正常启动。双方使用相同的游戏版本和补丁。
2. 从 [Releases](https://github.com/Blacktea0/frosthaven-lan/releases/latest) 下载 ZIP，**完整解压**到可写目录，不要只复制 EXE。
3. 选择启动方式：**普通双击** `frosthaven-lan.exe`，工具会直接启动游戏本体；**右键 → 以管理员身份运行**，工具会立即通过 Steam 启动游戏。已有游戏进程时会自动复用。
4. 等待出现 `LAN backend ready`。房主载入战役，从暂停菜单进入“多人游戏”开房。
5. 客户端选择“多人游戏 → 加入游戏”，输入房主 IPv4（例如 `192.168.1.20`，不带端口）；房主分配角色并确认。

**由于 Windows 权限限制，通过 Steam 启动的游戏进程需要管理员权限才能注入。** 如果游戏已由 Steam 启动，请关闭工具，再右键选择“以管理员身份运行”。普通权限适用于由工具直接启动游戏的方式。

网络需允许 **UDP 30000**。房主界面显示的 `localhost` 不能用于其他电脑连接，请使用房主实际 IP。仍需 Steam 身份初始化，不支持完全离线启动。

注入完成后可按 Enter 关闭工具，游戏会继续运行。失败时查看解压目录中的 `launcher-error.log`；更新补丁后请重启游戏。

## 开发环境需求

- Windows x64。
- Rust stable，`x86_64-pc-windows-msvc` 工具链。
- Visual Studio C++ Build Tools，包含 MSVC 和 Windows SDK。
- Windows .NET Framework C# 编译器：`C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe`。
- 系统 PowerShell，用于 ZIP 打包。

Harmony 2.3.3 已包含在仓库中。编译无需安装游戏；实际注入测试需要支持版本的游戏及 Steam 环境。

## 如何编译开发

在项目根目录执行，重建前先退出游戏。

```text
cargo xtask build           # 构建正式版到 dist/
cargo xtask package         # 生成正式版 ZIP 和 SHA256 到 artifacts/
cargo xtask run             # 构建并运行，按当前终端权限选择启动方式
cargo xtask run --pid <PID> # 使用已有游戏进程
```

只有一个运行版本：普通终端直接启动游戏，管理员终端通过 Steam 启动。请使用 `xtask` 生成完整运行包，`cargo build` 仅构建 Rust 部分。

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo xtask test-managed
```

提交使用英文 [Conventional Commits](AGENTS.md)，例如 `fix: wait for runtime assemblies`。
