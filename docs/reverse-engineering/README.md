# 逆向分析流程

目标为 Windows x64 / Unity Mono 构建，版本与哈希见根目录 [README](../../README.md)。所有安装文件检查均为只读；最终实现仅在进程内生效。

1. **识别引擎与依赖**：从 `Frosthaven_Data/Managed`、`MonoBleedingEdge` 和原生 Plugins 识别 Mono、EOS、Facepunch Steamworks、ENet。
2. **定位传输后端**：在 `Assembly-CSharp.dll` 中找到 `FFSNetwork`、`INetworkBackend`、`EOSNetworkBackend`、`EnetNetworkBackend`，比较构造签名和会话方法。
3. **排除无效配置路径**：`EditorProperties.Load/Save` 在发行版中为空，`UseLanNetworking` 未接入业务读取，不能仅靠 JSON 配置启用 LAN。
4. **确认注入点**：`FFSNetwork.<InitializeCrt>d__62.MoveNext` 直接创建 EOS 后端；两个后端构造签名相同。Harmony 替换构造调用，对已初始化且空闲的后端在 Unity 主线程切换，保留原连接检查回调。
5. **跟踪连接与序列化**：ENet 主机使用 UDP 30000，客户端将 sessionID 传给 Address.SetHost。既有邀请码输入直接进入此链路，因此 LAN 模式可复用为 IP 输入。检查 UserToken、ProtocolTokenUtils 和事件队列。
6. **定位语言入口**：读取 I2 语言源确认 12 种实际语言，定位两条词条。补丁拦截 I2.TryGetTranslation，并验证游戏 GLOOM 的上层查询入口；不修改语言表。

## 保留的材料

| 文件 | 内容 |
| --- | --- |
| [初始静态分析](static-analysis.md) | 首次分析时的发现与假设；不代表最新验证状态 |
| [network-il.txt](il/network-il.txt) | EditorProperties 与 ENet 后端 |
| [flow-il.txt](il/flow-il.txt) | FFSNetwork、会话及发送流程 |
| [states-il.txt](il/states-il.txt) | 初始化/加入协程与加入界面 |
| [target-il.txt](il/target-il.txt) | NetworkManager 默认配置 |
| [runtime-il.txt](il/runtime-il.txt) | 用户令牌、连接检查等运行时方法 |
| [protocol-il.txt](il/protocol-il.txt) | 协议编码和 ENet 事件轮询 |
| [localization-api.txt](il/localization-api.txt) | 本地化 API 的反射记录 |
| [localization-dump.txt](localization-dump.txt) | 游戏进程中导出的相关词条与语言列 |

IL 文件保留原始扫描输出，偏移和元数据 token 仅适用于当时的程序集。早期一次性 Python 分析脚本已移除，项目构建及运行均不依赖 Python。

`tools/Inspect.cs` 保留了只读反射查看器源码，用于检查游戏本地化接口。它不是完整反编译器；历史 IL 输出无法仅靠该工具重新生成。重新分析可使用 ILSpy，或自行扩展只读 IL 查看器。游戏更新后，应重新核对构造调用、协议结构、词条和哈希，再调整补丁。
