# Steam 头像与 LAN 传输静态分析

分析日期：2026-10-24。仅用 ILSpy 读取安装的托管程序集，未启动或注入游戏，未修改安装文件。反编译输出保留在系统临时目录，不纳入仓库。

Assembly-CSharp.dll SHA256：`20E5C317378426B0B3BF2833E4F54ED34677E062DAFCAEECEDD1B09340A606C5`，与当前支持版本一致。

最新状态：已实现两通道补丁，并完成下文“修复后的真实游戏验证”。此前各节中的“尚未修复”等描述保留为调查过程记录。

## 结论

原游戏已有完整的“本机获取 Steam 头像 → 图片分片 → 联机发送 → 远端重建 → UI 展示”路径，ENet 后端也能承载头像消息。局域网传输本身不要求接收端再查询发送者的 Steam 头像，也不依赖 Steam 叠加界面。

进一步对比注入补丁、EOS/ENet 实现及游戏自带 enet.dll 后，确认了切换后的直接故障：ENet 连接仅协商一个通道，头像却发送到通道 1，原生发送拒绝该通道，后端忽略返回值。下面的时序与重试问题属于额外薄弱点，不应取代这个确定的通道配置错误。尚未修复运行时代码或进行双机验证。

## 注入切换为什么导致其他玩家头像消失

`managed/Payload.cs` 的 `SwapConstructor` 将 EOS 构造替换为 ENet；`Pump` 在初始化已完成的情况下也会替换 `_backend`。两条路径均保留当前用户 token，没有修改头像获取、编码或 UI 方法，也没有修正 ENet 的连接通道数。

原来的 EOS 后端直接以消息的 channel 调用 EOS 发送 API，没有 ENet 的连接通道数问题。切换后实际执行：

```text
EnetNetworkBackend.JoinAsync
  → _enetInstance.Connect(address)
  → ENet.Host.Connect(address, 0, 0u)
  → enet_host_connect(host, address, channelCount = 0, data = 0)
  → 原生代码把 channelCount = 0 规范为 1
  → 连接只有通道 0

NetworkPlayer.StartLoadingAvatarCrt / SendAvatarDataToPlayer
  → NetworkDataStreamChannel.PlayerAvatar = 1
  → Peer.Send(1, ...)，或 Host.Broadcast(1, ...)
  → enet_peer_send 拒绝 channelID >= peer.channelCount
```

游戏实际携带的 Windows x64 enet.dll 反汇编证据（RVA，不依赖加载基址）：

- `enet_host_connect` 入口 `0x7690`；`0x76B0` 检查第三参数 R8 是否为 0，`0x76B3` 设默认通道数为 1，`0x76B8` 只在参数非零时采用显式通道数；`0x773D` 把通道数存入 peer。
- `enet_peer_send` 入口 `0x5230`；`0x5251` 扩展 channelID，`0x5255` 比较 peer 通道数，`0x5259` 在通道数小于或等于 channelID 时跳到失败返回，返回 -1。
- 托管 `ENet.Peer.Send` 仅把原生返回值是否为 0 转为 bool。`EnetNetworkBackend.SendProtolTokenToServer` 与 `SendProtocolTokenToClient` 未检查该 bool；广播路径同样没有向调用者提供逐连接发送失败结果。

因此，通道 0 上的身份、房间和游戏同步可以正常工作，通道 1 上的头像却没有成功发出。两端自己的头像仍可本机显示，另一端看不到。客户端头像在送达房主之前就失败，房主在 Main 通道转发的代码无法补救；房主发往客户端的头像也会失败。

这是启用原游戏 ENet 后端暴露的既有缺陷：补丁只替换了后端，没有补齐原实现的多通道配置。首要修复是让连接明确协商至少两个通道（如调用 `Connect(address, 2)`），并检查发送失败。另一方案是在 LAN 模式把头像也发送到 Main，但会改变头像与主消息的排队隔离，需要权衡。两端应统一使用修复版本，再验证互看与后加入。

## 原游戏调用链

1. `FH.Platforms.NewPlatformSteamUser.GetAvatar`：存在 `_avatarSprite` 时直接回调，否则解析 Steam ID，启动 `CreateAvatarCrt`。
2. `CreateAvatarCrt`：等待 `SteamClient.IsValid`，通过 `Task.Run` 调用 `SteamFriends.GetLargeAvatarAsync(steamID)`，等待任务完成，读取 `task.Result`。结果存在时转换为 Unity 纹理及 Sprite，缓存后回调。结果为空时不回调；此方法没有失败重试、超时或任务异常处理。
3. `NewPlatformUser.GetCurrentUserData`：取得本机头像后，把头像和身份信息一起交给调用者。`NewPlatformLayer.GetCurrentUserData` 负责等待平台可用并转发。
4. `PlayerRegistry.CreateNetworkPlayer`：注册玩家，本机玩家启动 `NetworkPlayer.StartLoadingAvatarCrt`；离线创建本机玩家的 `CreateMyPlayer` 也启动该协程。
5. `StartLoadingAvatarCrt`：仅为本机玩家加载头像，等待用户数据回调，将头像转成 128 × 128 的 RGB24 纹理，设置 `AvatarLoaded = true`。如果此时头像非空且 `FFSNetwork.IsOnline`，向其他玩家发送头像；否则结束，没有在该协程内安排补发。
6. `PlayerAvatarStreamingDataToken.GenerateTokensFromSprite`：编码 PNG、压缩，以每片最多 900 字节拆包，附带 PlayerID、Offset、Total 和纹理信息。
7. `FFSNetwork.SendTokenToClients` / `SendTokenToClient`：头像使用 `NetworkDataStreamChannel.PlayerAvatar`。`EnetNetworkBackend` 使用 `PacketFlags.Reliable` 发送；客户端广播通过 `ClientToClientToken` 交给房主转发。房主的这条转发路径将通道改为 `Main`。
8. `FFSNetwork` 的房主和客户端消息分发均识别头像 token，调用 `GHNetworkCallbacks.OnEvent(PlayerAvatarStreamingDataToken, ...)`，后者执行 `PlayerRegistry.GetPlayer(evnt.PlayerID)?.AvatarStreaming(evnt)`。
9. `NetworkPlayer.AvatarStreaming`：首片清空缓冲，逐片收集，末片触发解压与 Sprite 重建，设置 `AvatarLoaded = true`。
10. `UIMultiplayerUser.UpdateAvatarCrt` 等待加载后赋值 `userAvatar.overrideSprite`；`Update` 也会检测头像变化并刷新，所以晚到的有效头像仍有展示路径。

## 加入房间时的补发

`GHHostCallbacks.OnEvent(PlayerEntityRequestToken, ...)` 创建加入者，向客户端发送玩家 token 和槽位状态，然后遍历已有玩家，调用 `SendAvatarDataToPlayer` 给新加入者发送已取得的头像。

`SendAvatarDataToPlayer` 要求头像已经加载、非空且当前处于联机状态；条件不满足就直接返回。新加入者自己的头像由本机玩家的加载协程广播。

`FFSNetwork.IsOnline` 只检查后端是否运行且未关闭，不是 Steam 在线状态检查，因此不会因为使用 ENet 而天然阻止头像发送。

## 已确认的薄弱点与待验证推断

- **玩家不存在时丢包（代码事实）**：头像回调使用空条件调用，玩家未注册就忽略该片，没有待处理缓存。玩家 token 默认走 `Main`，头像走 `PlayerAvatar`。**跨通道的到达顺序导致头像提前到达，是合理故障候选，尚未通过双机复现确认。** Reliable 标志不能代替应用层的玩家注册确认。
- **接收过程不检查完整性（代码事实）**：看到末片就重建，没有先检查每片是否齐全，也没有在这一层请求重传。提前被忽略的分片不会因后续玩家注册而重新交付。
- **加载和补发缺少持续重试（代码事实）**：Steam 返回空结果时没有回调，任务失败没有恢复流程；发送条件不满足时直接返回。**本机有头像、远端没有头像时，应优先检查发送、接收和玩家注册时序；本机也没有头像时，应优先检查 Steam 获取。**
- **Steam 头像缓存不是按玩家索引（代码事实）**：`GetAvatar` 不论传入哪个 playerId，只要 `_avatarSprite` 已存在就返回该缓存。现有联机路径只获取本机头像，因此不能据此认定此次故障由缓存导致；未来不应直接用此方法批量查询远端玩家头像，否则可能返回本机头像。

## 建议修复与验证

首先修复 ENet 连接的通道协商，再验证原头像协议是否恢复。之后按实际验证结果考虑对未注册玩家的头像分片做有界内存缓存、注册确认后补发、有限重试和分片完整性检查，以及 Steam 获取的失败处理。全部诊断保存在内存并通过现有通信输出，不增加运行时日志文件。

验证至少覆盖房主与客户端互看、第三人后加入、头像延迟取得、断线重进和跨通道消息先后变化。静态分析与构建检查不能替代这些双机验证。

## 本地双进程传输验证

2026-10-24 使用 `tools/tests/enet_avatar_probe.py` 启动两个独立 Python 进程，直接调用游戏自带 enet.dll，在 IPv4 回环地址上建房与连接。没有启动游戏或 Steam API，没有注入。两端使用同一测试身份字符串，并读取现有 Steam 头像缓存中的 PNG（83,229 字节）；图片仅在内存中压缩分片、传输和校验，不复制到仓库。

| 配置 | 通道 0 身份消息 | 双方各自发送头像 | 双方接收头像 |
| --- | --- | --- | --- |
| 原默认连接参数 0 | 均收到 | 93 片失败，0 片成功 | 均未收到 |
| 明确协商 2 个通道 | 均收到 | 93 片成功，0 片失败 | 均完整收到，SHA-256 与输入一致 |

测试进程的身份字符串相同；其值不会影响 ENet 通道协商。默认配置已复现底层头像传输失败，因此本轮没有进入“不同身份 + 随机图片”的备用分支。

验证边界：脚本使用测试专用身份/图片帧格式，沿用可靠发送、头像通道 1、压缩及 900 字节分片，但没有调用游戏的 UserToken/PlayerAvatarStreamingDataToken 序列化、玩家注册与 Unity UI。这里证实的是两个独立实例之间的图片传输故障与双向恢复，不能据此声称两个同 Steam 账号的游戏实例已经完成头像展示验证。运行时补丁尚未修改。

## 真实游戏双实例复现

随后按用户明确要求，使用同一 Steam 账号启动两个实际游戏实例。临时目录中的启动器源码副本放开多实例启动限制，强制直接启动自己创建的进程，使用窗口模式；正式启动器源码未修改。首次启动遇到启动早期模块枚举 OS error 299，关闭该实例后，在临时副本中增加对此错误的重试并重新启动。两个新实例均报告 LAN 后端注入成功。

房主继续载入现有战役，通过暂停菜单创建 LAN 房间；客户端输入 `127.0.0.1` 成功加入，随后载入与房主相同的战斗场景。房主玩家列表显示两个相同账号名称的玩家：本机 Steam 图片头像正常，远端玩家为纯色人形占位头像。将一个角色分配给远端玩家后，该角色上方的玩家头像也仍是占位图；客户端已收到角色分配并进入场景。

由此真实复现了“相同账号可加入同一 LAN 会话，房主本机头像正常、远端头像缺失”。本次没有修改头像通道配置，因此尚未完成真实游戏中的修复前后对照；两通道修复的成功结果仍仅限上一节的独立 ENet 图片传输测试。

## 修复后的真实游戏验证

新增 `managed/LanChannels.cs`，用 Harmony 仅修改 `EnetNetworkBackend.JoinAsync` 中的单参数 `Host.Connect(Address)` 调用，改用 `Connect(Address, 2)`。补丁校验调用数量必须恰好为 1，签名不符时安装失败并保留原始异常；restore 会随同其他 Harmony 补丁移除它。不改动平台身份、图片内容或原头像协议，也不改动正式启动器的单实例限制。

完成构建后，把新运行 DLL 放入此前临时测试启动器目录，重新启动两个真实游戏实例，均确认 LAN 注入成功。同账号、同战役、`127.0.0.1` 加入，观察结果：

- 客户端成功加入并载入房主战斗场景。
- 房主底部玩家列表中，本机与远端玩家均显示真实 Steam 图片头像；修复前远端是黄色人形占位图。
- 角色分配弹窗中，两个玩家均显示图片头像。
- 将中间角色交给远端后，该角色上方显示远端图片头像，角色分配正常同步到客户端。

这次实测直接验证了此前复现故障已修复。可见头像的对照证据来自房主管理界面；客户端进入场景与控制归属也已观察确认。尚未覆盖两台物理电脑、不同账号、三人转发与重连场景。

检查通过：`cargo fmt --all -- --check`、`cargo test --workspace --locked`、`cargo xtask test-managed`、`cargo xtask package`。新增托管测试通过真实 Harmony patch/unpatch 验证通道数、其他 ENet 调用不受影响、方法形状不匹配时拒绝安装；编译和测试不依赖安装游戏程序集。打包在关闭旧实例后执行，之后才按用户要求进行手动双实例验收。
