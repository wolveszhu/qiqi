# E1a 联调问题清单

| 编号 | 场景 | 现象 | 根因 | 级别 | 修复提交 | 回归结果 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| E1a-001 | S3/S5/S6（冷启动主链路） | 启动后对话无流式、鼠标移动猫眼不跟随、空闲不睡、好感度变化窗口 A 无表情反馈 | `PetWindow.vue` 从未实例化 `useChatStream`/`useAffection`/`useSystemProbe`；`ChatInput @send`、`window:chat:toggle`、`SpeechBubble` 数据源均未接线 → MVP 主链路断开 | P0 | 重写 `PetWindow.vue` 接线（见下） | S3/S4/S5/S6 主链路打通 |
| E1a-002 | S5（好感度闭环） | 点击抚摸猫身未触发好感度+1 | `onAction("pet")` 仅 `playAction("click")`，未调用 `settleAffection({kind:"petStroke"})` | P1 | 同 E1a-001 提交（onAction 补 petStroke） | 抚摸计分生效（C5 日上限 20 内） |
| E1a-003 | M0（点击回弹）/ 契约 | 事件矩阵列 `pet:action` 但 Rust 端从未 emit | C2 穿透轮询仅检测"悬停"以驱动穿透，无鼠标按键下沿检测；前端当前靠 `.pet-root` 的 `@pointerdown` 直接 `playAction("click")` 兜底，M0 点击回弹可用 | P2 | 未修（兜底可用） | 转 E3/C2 补强：轮询补按键检测 emit `pet:action` 统一架构 |
| E1a-004 | D1.5（文件拖入多模态） | `window:drag:drop` 事件无 emit / 无订阅，拖文件入窗口无效 | D1.5 未实装；仅契约文档 `接口契约.md` 列了该事件 | P2 | 未修 | 转 F4/E3（D1.5 范围） |
| E1a-005 | E1a.2 状态机核对 | 窗口 B 看板（AffectionPanel）未订阅 `affection:changed` 实时刷新，仅打开时读取 | `SettingsWindow.vue` 未调用 `useAffection`/订阅；矩阵要求 A+B 双订阅 | P2 | 未修 | 转 E3 打磨（看板实时性） |

## 核对结论（E1a.1 事件矩阵）

- 9 类事件中 **8 类已对齐**（payload DTO 双端 camelCase 一致、unlisten 清理齐全）：`probe:cursor:move` / `probe:idle:changed` / `probe:foreground:changed` / `chat:stream:delta|done|error` / `affection:changed` / `settings:dnd:changed` / `window:chat:toggle` / `proactive:show`。
- **1 类漂移**：`window:drag:drop`（E1a-004，未实装）。
- `pet:action` 事件存在但 Rust 未发出（E1a-003，前端 DOM 兜底可用）。
- Tauri 2 命令参数约定确认：前端 camelCase（如 `sessionId`）→ 后端 snake_case（`session_id`）自动映射，`chat_send_stream`/`chat_abort` 绑定正确，无命名缺陷。

## 修复提交

- `fix(ui): E1a 接通 MVP 主链路（PetWindow 接线 chat/affection/probe + 对话气泡 + 输入框切出 + 抚摸计分）`
