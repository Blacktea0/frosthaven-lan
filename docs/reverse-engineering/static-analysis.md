后续实现采用进程内注入。使用说明见 [README.md](../../README.md)。下文保留首次静态分析结论。

# Frosthaven 局域网联机静态分析

分析日期：2026-10-23
目标：<SteamLibrary>\steamapps\common\Frosthaven
范围：读取安装文件、解析 .NET 元数据与 IL；未启动游戏，未修改安装目录。

## 结论

当前安装包已经包含可用作 LAN 传输的 ENet 后端和原生 enet.dll。发行版初始化网络时直接实例化 EOS 后端，LAN 开关没有接入此流程。因此存在改动很小的实现路径，但尚未经过运行时和双机验证，不能据此宣称已经能够联机。

## 直接证据

- 引擎：Unity，Mono 托管程序集，核心逻辑位于 Frosthaven_Data/Managed/Assembly-CSharp.dll。
- 程序集 SHA256：20E5C317378426B0B3BF2833E4F54ED34677E062DAFCAEECEDD1B09340A606C5。
- FFSNetwork.<InitializeCrt>d__62.MoveNext，MethodDef 52248（0x0600CC18），当前反汇编偏移 0xD0：newobj FFSNet.Backend.EOSNetworkBackend::.ctor，然后写入静态 _backend。
- ENet 构造函数 MethodDef 34466，EOS 构造函数 MethodDef 34493；两者签名字节完全相同：20020112b52412b5b8。均接收现有 NetworkManager 与 UserToken。
- EnetNetworkBackend.HostAsync：创建 ENet Host，端口硬编码 30000，玩家上限来自 NetworkManager.MaxPlayers；把 SessionID 写为 localhost。
- EnetNetworkBackend.JoinAsync：端口硬编码 30000，把传入 sessionID 直接交给 ENet.Address.SetHost，再执行 Host.Connect。
- NetworkManager 构造函数默认 maxPlayers 为 4，实际场景序列化配置尚未检查。
- Frosthaven_Data/Plugins/x86_64/enet.dll 存在，PE 导出包含 enet_host_create、enet_host_connect 等 ENet API。
- EditorProperties 的 UseLanNetworking 默认 false；当前程序集未发现对该 getter 的业务读取。EditorProperties.Load 和 Save 都仅有 ret，创建 EditorProperties.json 不会使其生效。
- UIMultiplayerJoinSessionWindow.JoinSession 把输入内容 ToUpper 后交给 FFSNetwork.JoinSession，数字 IPv4 地址不受影响。ValidateCode 只比较最小长度；构造默认最小长度为 1，但场景/预制体的输入字符限制尚未检查。
- 用户标识仍来自 NewPlatformLayer.GetCurrentUserData；切换后端不等于移除 Steam/EOS 初始化或实现完全离线启动。

## 最小实现方案

1. 在程序副本上，将 FFSNetwork.<InitializeCrt>d__62.MoveNext 中创建 EOSNetworkBackend 的构造调用替换成 EnetNetworkBackend 构造调用。两者签名一致，可保留周围的参数传递、回调绑定和同步逻辑。
2. 两台电脑使用相同游戏版本及相同改动；房主从现有多人入口创建游戏。
3. 客户端在加入界面输入房主实际 IPv4，例如 192.168.1.20，不附加 :30000。当前后端自行固定端口。
4. 房主需要允许游戏接收 UDP 30000；两台电脑需可互通，避免访客 Wi-Fi/AP 客户端隔离。
5. 若输入框拒绝点号或截断 IP，再调整输入组件的 characterValidation/contentType/characterLimit。当前仅发现潜在资源配置问题，不能断言必需修改。

此方案无需重写游戏消息、行动同步和存档流传输，也无需替换 steam_api64.dll。仍使用原有平台用户身份。

## 建议的正式实现

使用可开关的补丁：LAN 模式创建 EnetNetworkBackend，普通模式保留 EOS；加入界面提示输入主机 IP；房主显示实际网卡地址而非 localhost。默认多网卡时允许选择地址。端口可以先保留 30000，再考虑统一配置。补丁应校验原文件哈希或方法特征、保留原文件，并能恢复；游戏更新后重新检查。

## 必须验证的边界

- 游戏日志应出现 FFSNetwork finished initialization with backend，类型应为 EnetNetworkBackend。
- 房主开启多人后，按游戏 PID 检查 UDP 30000 监听。
- 第二台电脑输入房主 IP，确认连接、玩家身份、角色分配、进入场景、回合推进、读档同步、断线重连。
- 检查是否仍有平台登录/在线状态检查阻挡多人入口；切换传输层不会自动处理这些检查。
- 确认所有参与者有不同的平台用户标识，版本和模组一致。
- 单机静态分析不能验证网络传输稳定性、同步兼容性和完整离线流程。

## 分析文件

network-il.txt：EditorProperties 与 ENet 后端相关 IL。
flow-il.txt：FFSNetwork 和会话相关调用。
states-il.txt：网络协程状态机与加入界面 IL。
target-il.txt：NetworkManager 默认配置。
早期一次性分析脚本已清理；原始 IL 输出保留在本目录的 il/ 下。该段为首次静态分析时的文件记录。

当前没有生成或安装游戏补丁，也没有修改防火墙。
