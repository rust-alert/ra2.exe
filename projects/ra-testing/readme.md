# ra-testing

`ra-testing` 是测试支撑 crate，不是游戏运行时，也不包含原版资源。

## Headless

`HeadlessCase` 经 `ra-engine` 驱动对局，通过 `GameCommand` 入队、`Session::tick` 推进，并产出包含 tick、状态摘要、胜负和
`RenderSnapshot` 的 `HeadlessObservation`。测试不创建窗口、不初始化 wgpu，也不读取用户安装目录。

`standard_duel()` 提供两辆 `MTNK` 的合成遭遇战夹具，是 `alpha-skirmish-v1` 的最小战斗前身，用于命令、移动、攻击、胜负与确定性回归。
`mcv_deploy_open()` 播种盟军 MCV 与冻结初始资金，用于部署/经济 headless。完整竖切清单见 `alpha_skirmish_v1()`。

```shell
cargo test -p ra-testing
```

## GUI 自动化

`GuiAutomationPlan` 是平台无关的测试意图：启动目标程序、等待窗口、输入、截图、检查标题并退出。当前只定义计划模型，尚无完整 OS
执行器。

`standard_duel_gui_plan` 固化首个 Alpha 合成场景步骤，并配合 `TestStatus` 解析 `RA2_TEST_STATUS_PATH` 旁路（`tick` /
`hash` / `outcome` / `selected`）。

Windows 第一版执行器应独立放在测试工具或专属路径中，使用 UI Automation 驱动窗口和输入，并在 CI 的带桌面会话环境运行。它不能进入
`ra-desktop` 的正常依赖图。截图基线只覆盖冻结的测试场景，须固定窗口尺寸、GPU 后端、字体缩放、内容目录和测试时钟；世界正确性仍以
headless 的状态摘要与快照断言为准。

## 下一步

1. 用 `alpha_skirmish_v1` 驱动 MCV、建造、采矿与生产的 headless 剧本，逐步取代仅互殴的验收。
2. 在 `ra-desktop` 增加仅测试构建可用的启动场景和可读状态接口：`--features test-harness`，`--test-scene=duel` /
   `RA2_TEST_SCENE`，以及 `RA2_TEST_STATUS_PATH` 旁路文件。
3. 实现 Windows GUI 执行器，先覆盖启动、选中、移动、攻击、胜负、截图和退出。
4. 把 headless 回归放入所有平台 CI，把 GUI 自动化放入 Windows 带桌面会话的独立作业。

许可证：MPL-2.0。
