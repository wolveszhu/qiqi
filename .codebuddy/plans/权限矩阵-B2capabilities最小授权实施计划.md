# 权限矩阵子计划（B2：capabilities 最小授权）

| 项 | 内容 |
| :--- | :--- |
| 父计划 | [.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md](桌宠-任务拆分与开发顺序规划.md) §三 阶段 1 / 任务 B2 |
| 规则依据 | `.codebuddy/rules/`：code-standards §5（边界隔离与最小授权精神）、development-guide（查官方文档、先设计后落地）、lint-rules（门禁）；权限规格来源：`桌宠.md` §3（窗口/系统能力清单）+ 父计划 §七 命令分组 |
| 范围边界 | **只做 capabilities 授权设计**：core 窗口/事件权限 + 插件权限策略。不含：插件依赖引入（随 C2 等任务）、命令实现（B5/C2）、窗口定义（B1） |
| 现状 | `capabilities/default.json` 仅 `core:default` + `opener:default`；双窗口 label 约定 `main`/`settings`（B1 子计划已定）；C2 子计划已列其所需权限子集（C2.0） |
| 状态 | **已完成**（2026-10-21）——执行结果见文末「执行结果与计划对应情况」 |

---

## 一、范围与依赖状态说明

| 项 | 说明 |
| :--- | :--- |
| 前置 A2（未落） | 契约文档未产出——本计划直接引用父计划 §七 命令分组 + C2 子计划清单设计矩阵；A2 文档落地后将本计划产出的**权限映射表并入其「权限映射」小节**（单一真源） |
| 与 C2.0 去重 | C2 子计划前置含权限子集：B2 先行则 C2.0 仅剩实现验证；本计划按 B2 先行编写，覆盖面为全量（不止 C2 所需） |
| **勘误登记（可追溯）** | 父计划 B2 验收标准"新增命令调用不再因权限失败"易误导：Tauri 2 中**应用自定义命令（`invoke_handler` 注册的 `window_*` 等）不经 capability 门控**，天然可调；capability 只管 **core API（前端 JS 调用窗口/事件方法）与插件前端 API**。B2 的真实价值：① 前端直接使用的 core API 可用（如 `start_dragging`、`hide`、`listen`）；② 最小化、禁大撒把；③ 插件引入时权限随阶段追加。执行完成后回写父计划该行表述 |

---

## 二、任务分解（B2.1–B2.5）

| ID | 任务 | 具体内容 | 依赖 | 验收标准 |
| :--- | :--- | :--- | :--- | :--- |
| B2.1 | 权限矩阵设计 | 逐条映射：前端能力（现役 + C1/C2/D 阶段可预见）→ 所需权限标识符 → 适用窗口（main/settings）；产出矩阵表（§三），**先设计后落地**（development-guide 工作流） | - | 矩阵表覆盖所有已规划的 JS 侧 core API 调用 |
| B2.2 | capabilities 落地 | `capabilities/default.json`：显式声明 `"windows": ["main", "settings"]`（不依赖隐式全窗口）；权限按 §三 清单逐条写入并**注释分组**（窗口操作/事件/插件） | B2.1 | dev 启动无权限报错；逐项手工验证通过（§五） |
| B2.3 | 最小化复查 | 禁止项自查：不用 `core:window:allow-unchecked` 类大撒把；`core:window:default` 等集合型基线仅保留必要集合；逐条问"这个权限对应哪个用户可见能力" | B2.2 | 无冗余权限；每条可追溯到矩阵行 |
| B2.4 | 插件权限策略 | 裁决：`global-shortcut`/`autostart` 均**仅 Rust 侧使用**（C2.7 注册与托盘）→ 不写其 JS 权限条目；`store/dialog/fs` 未来由 D 阶段任务按需追加——登记"按阶段引入清单"，禁止一次性预授权 | B2.1 | 策略写入矩阵文档；capabilities 中无未启用插件条目 |
| B2.5 | 回写与登记 | 权限映射表登记（待 A2 契约文档产出时并入）；同步 `CODEBUDDY.md` 配置表；回写父计划 B2 行（含 §一 勘误表述修正） | B2.2–B2.4 | 文档/配置/父计划三者一致 |

---

## 三、权限矩阵（目标形态，写入 capabilities 的依据）

### 3.1 core 权限（两个窗口共用，单文件策略）

| 权限标识符 | 消费方（前端） | 支撑能力 | 窗口 | 归属任务 |
| :--- | :--- | :--- | :--- | :--- |
| `core:default` | 框架基线集合 | 基础生命周期（保留现状） | main, settings | 既有 |
| `core:window:allow-start-dragging` | B1.4 标题栏 `data-tauri-drag-region` / C2.5 猫身拖拽 | 拖拽移动窗口 | main, settings（B 均为自绘标题栏） | B1/C2 |
| `core:window:allow-hide` / `allow-show` | TitleBar 关闭（B1.4 临时通道）、快捷键显隐 | 显隐窗口 | main, settings | B1/C2 |
| `core:window:allow-set-ignore-cursor-events` | （备用：前端直调路径；主路径走 Rust 命令免授权） | 穿透切换 | main | C2（预留） |
| `core:window:allow-set-focus` | 唤起窗口时聚焦 | 托盘/快捷键唤起体验 | main, settings | C2 |
| `core:event:allow-listen` | `useSystemProbe` 订阅 probe 事件、`window:chat:toggle`、chat 流事件 | 事件订阅 | main, settings | B5/C2/C4 |
| `core:event:allow-emit` / `allow-unlisten` | 前端事件通信 | 事件发送与清理 | main, settings | B5 |

### 3.2 插件权限

| 插件 | 状态 | 决策 |
| :--- | :--- | :--- |
| `opener` | 已启用 | 保留 `opener:default`（G3 外链说明用） |
| `global-shortcut` / `autostart` | C2.7 引入，仅 Rust 侧 | **无 JS 权限条目**；若未来 D2 需 JS 侧改键再追加 |
| `store` / `dialog` / `fs` | 未引入 | **不预授权**；引入任务的子计划中逐项申请 |

### 3.3 窗口拆分策略（登记决策）

MVP 用**单文件 `default.json`、显式 `windows: ["main","settings"]`**；若后续出现"某权限仅限单窗口"的安全需求（如 settings 才能触发某敏感 API），再拆 per-window capability 文件（`capabilities/settings.json` 等，规范 §4 已预留）——当前两窗口能力面一致，拆分无收益。

### 3.4 自定义命令授权说明（勘误 #1 的展开）

`window_*` / `settings_*` / `pet_*` / `chat_*` / `memory_*` / `probe_*` 均为应用命令：**无需 capability 条目**，`invoke_handler` 注册即可被任一窗口调用。若未来需要"按窗口门控自定义命令"，须用 Tauri 2 的应用级权限（`build.rs` + `permissions` 模块自定义权限）——MVP 不做，登记为演进路径。

---

## 四、文件清单（本计划允许的 diff 范围）

| 文件 | 操作 | 内容 |
| :--- | :--- | :--- |
| `src-tauri/capabilities/default.json` | MODIFY | §三.1/3.2 权限清单 + 注释分组 + 显式 windows 数组 |
| `docs/design/接口契约.md` | MODIFY（若 A2 已产出）或登记待并入 | 「权限映射」小节 = §三矩阵 |
| `CODEBUDDY.md` | MODIFY | 配置表同步权限矩阵要点 |
| `.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md` | MODIFY | B2 行回写（含勘误表述修正） |

**不动**：任何 Rust/前端代码、tauri.conf.json（B1 范围）、插件依赖（C2 范围）。

---

## 五、验证门禁与 DoD

- [ ] `pnpm tauri dev` 启动控制台**零权限告警**（Tauri 2 会在越权调用时打印被拒的权限名）
- [ ] 逐项手工验证（每条对应矩阵行）：
  1. 窗口 B 标题栏拖拽移动生效（`start-dragging`）
  2. TitleBar ✕ 隐藏窗口、进程存活（`allow-hide`）
  3. 前端 `listen("probe:cursor:move")` / `emit` 测试事件收发成功（`core:event:*`）
  4. 越权负样例：前端调用一个**未授权** API（如 `window.setSize`）→ 控制台明确权限拒绝（证明门控生效，而非全通）
- [ ] 最小化复查：无 `*` 通配、无未启用插件条目、每条权限有矩阵行背书
- [ ] `capabilities/default.json` 与 `CODEBUDDY.md`、契约文档三处一致
- [ ] 冷眼三问：① windows 数组是否显式（无隐式全窗口）？② 有无"为了省事"多加的权限？③ 勘误表述是否已回写父计划（避免后续任务再被误导）？
- [ ] 完成后勾选父计划 B2 行状态

## 六、风险与回退

| 风险 | 对策 |
| :--- | :--- |
| 权限标识符拼写错误 / Tauri 2 版本演进导致标识符变化 | 实现前核对官方 capabilities 文档对应版本；dev 控制台的"unknown permission"报错即修；逐条验证而非整批提交 |
| 授权不足导致 C2/C4 阶段反复补权限 | 矩阵已按"现役 + 可预见"设计；后续任务引入新前端 API 时按流程追加（子计划内申请） |
| 过度授权（安全红线） | B2.3 最小化复查 + 负样例验证；禁通配禁预授权 |
| 单文件双窗口策略未来成为安全短板 | 拆分路径已登记（per-window capability），成本一处配置 |
| 自定义命令被误以为需要授权而白白排查 | 勘误 #1 写入父计划与 CODEBUDDY.md，杜绝后续排查浪费 |

## 七、执行顺序

B2.1（矩阵设计）→ B2.2（capabilities 落地）→ B2.3（最小化复查）→ B2.4（插件策略登记）→ 逐项验证（含负样例）→ B2.5（回写登记）→ DoD 收口。

---

## 执行结果与计划对应情况（2026-10-21）

| 计划任务 | 实际结果 | 对应 |
| :--- | :--- | :--- |
| B2.1 矩阵设计 | 全量矩阵（8 条 core + 插件策略）并入 `docs/design/接口契约.md` §4.1–4.3（A2 已产出，按计划"并入权限映射小节"落实，单一真源） | ✅ 对应 |
| B2.2 capabilities 落地 | `capabilities/default.json`：显式 `"windows":["main","settings"]`（原仅 `["main"]`）；权限 = core:default + opener:default + window×5 + event×3；description 承载分组说明与矩阵指针 | ✅ 对应 |
| B2.3 最小化复查 | 无通配、无未启用插件条目、每条权限可追溯矩阵行；`core:window:allow-set-ignore-cursor-events` 标注"前端直调预留、主路径 Rust 命令"（B1 已按 Rust 命令实现显隐，未动用 JS API） | ✅ 对应 |
| B2.4 插件策略 | global-shortcut/autostart 仅 Rust 侧（C2.7）无 JS 条目；store/dialog/fs 不预授权——策略入契约 §4.2 与 CODEBUDDY.md | ✅ 对应 |
| B2.5 回写 | 父计划 B2 行（✅ + 勘误表述修正："命令不再因权限失败"→"core API 可用且最小化"）；CODEBUDDY.md 权限行同步矩阵要点；本计划执行结果 | ✅ 对应 |

**勘误#1 落实**：自定义命令（window_* 等）不经 capability 门控——已写入父计划 B2 行、契约 §4.1、CODEBUDDY.md 三处，杜绝后续任务被误导做无效排查。

**门禁**：`pnpm typecheck` / `build` / `test` / `lint` / `format:check` + cargo 四门禁全绿（capabilities 为配置文件，不影响编译）。

**待用户手工验证**（`pnpm tauri dev`，GUI 项）：① 启动控制台零权限告警；② 窗口 B 标题栏拖拽生效（start-dragging）；③ TitleBar ✕ 隐藏进程存活（当前 Rust 命令路径，allow-hide 为 C2 前端直调预留）；④ 负样例：控制台调未授权 API（如 `window.setSize`）应被明确拒绝——证明门控生效。
