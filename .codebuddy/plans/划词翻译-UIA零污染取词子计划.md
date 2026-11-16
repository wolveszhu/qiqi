# 划词翻译：UIA 零污染取词子计划

> 状态：待执行（v1.0 · 2026-11-16）
> 目标：手势探测与取词链路**永不写入/清空用户剪贴板**，彻底消除「划词后复制粘贴像失效了」的问题。
> 上游计划：`全局划词翻译-悬浮工具条与译文浮窗实施计划.md`（原设计即要求取词前后剪贴板一致）
> 推翻的裁决：`接口契约.md:138-157` 与项目记忆「划词翻译剪贴板决策」（2026-11-11）

---

## 1. 背景与问题

### 1.1 用户反馈（原文）

> 关于「划词翻译存在 BUG：划词或者双击时如果复制了选中的文本，那么用户在执行复制粘贴操作时会永远无法进行操作，应该是划词翻译永远不复制文本进行翻译才最好」。

### 1.2 根因分析

当前 `src-tauri/src/probe/selection.rs` 的 `has_selected_text():480-521` 在**每一次**双击或拖选时执行：

```rust
let previous = clipboard.get_text().ok();
let _ = clipboard.clear();          // 哨兵：无条件清空用户剪贴板
simulate_ctrl_c();                  // 注入 Ctrl+C
// 命中 → 选中词留在剪贴板；未命中 → restore_text(previous)
```

五条叠加路径造成「复制粘贴永远不对」的主观体验：

| # | 路径 | 后果 |
| :-- | :--- | :--- |
| 1 | `restore_text():357-370` 在 `previous=None` 时执行 `clipboard.clear()` | 剪贴板里是**图片/文件**时 `get_text()` 失败 → 用户内容被彻底清空 |
| 2 | `has_selected_text():510-515` 命中后故意留选中词 | 用户已复制的内容被划选词**净覆盖** |
| 3 | 下一次手势再次 `clear()`，若 Ctrl+C 未命中（空白处双击、拖窗口/图标、UIPI 拦截） | `restore_text` 把「清空前内容」（上次选中词）写回，用户真实内容**永久丢失** |
| 4 | `start_selection_hook():221-248` 是全局 `WH_MOUSE_LL` 钩子，任何第三方应用的拖选/双击都置位；`poll_selection` 每 100ms 消费 | 高频触发 clear + Ctrl+C，用户日常操作持续被改写 |
| 5 | `has_selected_text()` 不调 `restore_foreground()`（对比 `capture_with_wait():537-563`） | 100ms 延迟 + 快速切窗时 Ctrl+C 发给错误窗口，放大清空/覆盖 |

### 1.3 现状与目标的根本冲突

- 现状：取词**唯一路径是剪贴板法**，注释明说「UIA `TextPattern` 主路径待接入」（`selection.rs:13-14`、`translate.rs:58`）。
- `Cargo.toml:34` 的 `windows = "0.58"` 仅启用 5 个 Win32 feature，**没有 UIAutomation**；`:44` 的 `arboard = "3"` 仍保留用于用户主动复制译文。
- DTO 已预留 `via: "uia" | "clipboard"`（`dto.rs:284-285`、`types.ts:59`），但无实现。

---

## 2. 已确认决策（用户 2026-11-16 选择）

| 决策项 | 用户选择 | 对设计的硬约束 |
| :--- | :--- | :--- |
| 取词方式 | **零污染：接 UIA 只读选区** | 手势阶段不 `clear()`、不注入 `Ctrl+C`；用 UI Automation `TextPattern` 读取选区；需为终端/远程桌面/自绘 UI 等 UIA 失效场景给出明确兜底 |
| 工具条「复制」按钮 | **保留** | 用户主动点击时写剪贴板是合规行为，不在禁止范围；探测链路仍不得碰剪贴板 |
| 工具条弹出门控 | **UIA 只读 bool 判定有无选区** | 不取文本、不碰剪贴板；配合既有 500ms 去重（`SELECTION_DEDUPE_MS`）与点外部收起抑制误弹 |

**统一定义「零污染」**：从手势发生到工具条消失的完整链路中，应用**不调用** `Clipboard::clear()`、`Clipboard::set_text()`、`simulate_ctrl_c()`；唯一允许写剪贴板的入口是用户主动点击「复制」/「复制译文」（`translate_copy_result`）。读取用户剪贴板也**不在探测阶段发生**；兜底若需读取，必须在用户明确动作之后。

---

## 3. 旧裁决推翻与文档同步

本计划**推翻** 2026-11-11 的裁决，执行阶段须同步处理：

1. **项目记忆** `C:\Users\stimmer\.codebuddy\projects\d-qiqi-qiqi\memory\feedback_clipboard.md`：内容是「命中文本时选中词永久留在剪贴板，工具条消失不清除」。用户本次明确反转，按记忆规则应**删除该条**（不是改标题），并从 `MEMORY.md` 移除索引行。
2. **`docs/design/接口契约.md:138-157`**：改写「文本手势选中词永久留剪贴板、非文本手势才还原」为新的零污染契约，保留 UIA 已知限制的说明并补充兜底策略。
3. **三份历史计划**：`全局划词翻译-悬浮工具条与译文浮窗实施计划.md:33,110,297,390,392`、`全局划词翻译-开发子计划_4da564a2.md:31,61`、`全局划词翻译-开发子计划_476624a2(未完成).md:37,55` 本来就要求「取词前后剪贴板完全一致」。本计划是**回归原设计**，在文首注明「恢复 2026-11-11 之前的取词契约」，不逐行修改这几份历史文档。
4. **`src-tauri/src/window/manager.rs:86-95`** 的 `hide_translator` 注释「剪贴板里的选中词原样保留」需改为「探测与取词全程不读写剪贴板」。

---

## 4. 技术设计

### 4.1 总体架构

```
全局 WH_MOUSE_LL 钩子（现有，只置位）
        │  O(1) 标志：DOUBLE_CLICK / PENDING / DISMISS
        ▼
100ms scheduler 线程（现有）
        │  poll_selection()
        ▼
UIA STA 工作线程（新增，专用）
        │  ① 只读 bool：焦点元素有无文本选区 → 决定是否 emit 工具条
        │  ② 只读 text：GetSelection() → 内存缓存（不写剪贴板）
        ▼
前端 TranslatorWindow
        ├─ 点「翻译」→ translate_execute(内存缓存文本)
        ├─ 点「复制」→ translate_copy_result(原文)   ← 唯一合法写剪贴板
        └─ 点外部/Esc → hide_translator → 清内存缓存
```

### 4.2 UIA 线程模型（关键）

- UIA COM 需在 **STA** 中调用（`CoInitializeEx(..., COINIT_APARTMENTTHREADED)`），并有消息泵支撑跨线程 COM 调用；不能直接在 100ms scheduler 线程或钩子回调里做。
- 新增专用 UIA 线程：启动后完成 COM 初始化并保持 `GetMessageW` 消息循环；对外提供两个串行请求（通过 `std::sync::mpsc` + 一次性 reply channel）：
  - `HasSelection`（只读 bool，超时 ≤ 50ms）
  - `ReadSelection`（只读文本，超时 ≤ 80ms）
- 所有 UIA 调用带**硬超时**；超时即视为不可用，走 §4.4 兜底，不阻塞 scheduler。
- `windows` crate 0.58 的 UI Automation binding（feature 名、接口路径）**以 windows-rs 0.58 官方文档为准**；执行时先用 `cargo check` 验证 feature 与 API，不凭记忆写死。
- 标准调用链（概念级）：`CUIAutomation` → `IUIAutomation::GetFocusedElement` → `GetCurrentPattern(TextPattern)` → `IUIAutomationTextPattern::GetSelection` → `IUIAutomationTextRange::GetText`。

### 4.3 取词链路（成功路径）

1. 手势结束后，`poll_selection` 不再调用 `has_selected_text()`（函数名与语义一并废弃），改为发 `HasSelection` 给 UIA 线程。
2. 有选区 → 立即发 `ReadSelection`，把文本存进 `GestureSelection { text, selection_key, at }` **内存槽**（替代原 `GestureClipboard`）。
3. emit `translate:selection:show`，前端弹工具条（compact）。
4. 用户点「翻译」→ `translate_capture_selection` 从内存槽按 `selection_key` 取文本，**不读写剪贴板**；内存槽已失效（超时/被新选区覆盖）时返回明确 `AppError::Probe` 提示，不降级到 Ctrl+C。
5. 工具条收起（`hide_translator`）只清内存槽与 bounds；`capture_with_wait()` 等在本次改造中**删除或改为仅内部使用**，不得再被探测/取词链路调用。

### 4.4 兜底矩阵（UIA 不可用的明确策略）

| 场景 | 判定 | 行为 | 是否碰剪贴板 |
| :--- | :--- | :--- | :--- |
| UIA 可用，有选区 | `HasSelection=true` | 正常弹工具条，内存缓存文本 | 否 |
| UIA 可用，无选区 | `HasSelection=false` | 不弹（或复用既有 DISMISS 收起） | 否 |
| UIA 不可用（超时 / 无 TextPattern / 终端 / 远程桌面 / 自绘 UI） | `HasSelection=None` | **不弹工具条**；在探针状态里记一次 `uia_unavailable` 诊断，日志可查 | 否 |
| 用户仍想翻译（UIA 失效场景） | — | 用户自行 `Ctrl+C` 后，用工具条提供的「手动粘贴」输入框（若本期不做输入框，则在 FAQ/提示中给出「复制文本到设置/对话窗口处理」的替代路径） | 仅用户自己复制；应用不读不写 |
| 用户主动点「复制」/「复制译文」 | — | 走 `translate_copy_result` 写剪贴板 | 是（用户主动，唯一合法入口） |

> 红线：**任何情况下都不再注入 `Ctrl+C`、不再清空/覆盖用户剪贴板**。宁可少弹工具条，也不做静默改写。

### 4.5 时序图（正常路径）

```
用户拖选  →  钩子置 PENDING
          →  100ms 后 poll_selection
          →  UIA 线程 HasSelection  ──true──▶ ReadSelection
          →  内存槽写入 text + selection_key
          →  emit translate:selection:show（compact）
用户点「翻译」
          →  translate_capture_selection(key) 从内存槽取文本
          →  translate_execute（HTTP，不碰剪贴板）
          →  translate_popup_show(compact=false) 清内存槽
用户点「复制」
          →  translate_copy_result(text)  ← 唯一写剪贴板
```

### 4.6 剪贴板生命周期对照

| 阶段 | 改造前 | 改造后 |
| :--- | :--- | :--- |
| 手势探测 | `clear()` + `simulate_ctrl_c()` + 命中留选中词 | UIA 只读 bool / 只读文本，剪贴板完全不动 |
| 工具条显示 | 选中词在剪贴板里，`gesture_captured_text()` 复用 | 文本在内存槽，不读剪贴板 |
| 点「翻译」 | 复用剪贴板或再走 `capture_with_wait()`（再注入一次） | 内存槽直接取；失效则报可行动错误，不降级 |
| 工具条消失 | 只丢内存状态，选中词留在剪贴板 | 清内存槽，剪贴板从未被改 |
| 点「复制」 | `write_clipboard_text()` | 保持不变（用户主动） |

---

## 5. 逐文件改动清单

| 文件 | 位置 | 改动 |
| :--- | :--- | :--- |
| `src-tauri/Cargo.toml` | 34 | `windows = "0.58"` 增加 UI Automation 所需 feature（**以 windows-rs 0.58 文档核实后填写**）；`arboard` 保留，仅用于主动复制 |
| `src-tauri/src/probe/selection.rs` | 全文 | 删除 `has_selected_text()` 的 `clear()` + `simulate_ctrl_c()`；`GestureClipboard` 改为 `GestureSelection`（内存文本，不备份剪贴板）；删除 `still_owns_clipboard`、`restore_text`、`capture_with_wait`（或降级为未使用）；`capture_selection` 改为从内存槽取；新增 UIA STA 线程与 `HasSelection`/`ReadSelection` 请求；`poll_selection` 改调 UIA 判定；更新模块头注释与 `:13-14` 的「剪贴板法唯一路径」 |
| 同上 | 578-659 单测 | 删除固化「清空哨兵 / 留选中词」的断言，新增「探测前后剪贴板逐字节一致」等测试（见 §7.1） |
| `src-tauri/src/probe/scheduler.rs` | 125-128 | `poll_selection` 调用方式不变；确认 UIA 线程不阻塞 100ms 周期（超时上限纳入预算断言） |
| `src-tauri/src/commands/translate.rs` | 58-71 | 去掉「UIA 待接入；当前为剪贴板法」注释；`translate_capture_selection` 改为从内存槽取；`translate_copy_result` 保持不变 |
| `src-tauri/src/window/manager.rs` | 86-95 | `hide_translator` 清内存槽（新函数名），改注释；不再有「剪贴板原样保留」口径 |
| `src-tauri/src/db/models/dto.rs` | 284-285 | `via` 枚举保留 `"uia" \| "clipboard"` 兼容；正常路径返回 `"uia"`；若未来加手动粘贴入口再用 `"clipboard"` |
| `src/api/types.ts` | 59 | 与 Rust DTO 对齐（若不变则不动） |
| `src/api/translate.ts` | 31-64 | 命令签名不变；可新增 `translate_has_selection` 诊断命令（可选） |
| `src/views/translate/TranslatorWindow.vue` | 106-147, 176-195 | 点「翻译」仍调 `captureSelection`；点「复制」保留；错误提示新增 UIA 不可用文案 |
| `src/views/translate/SelectionToolbar.vue` | 全文 | 按钮与 emit 结构不变；可不改 |
| `src/i18n/locales/{zh-CN,en-US}.ts` | translate.toolbar.* | 复核 `selectionCopied` 措辞；新增/替换一条「划词不会改变剪贴板」的提示与 UIA 不可用提示（**中英成对**） |
| `docs/design/接口契约.md` | 138-157、相关取词段落 | 改写剪贴板裁决为零污染契约，保留 UIA 限制与新兜底矩阵 |
| `docs/design/项目目录结构规范.md` / `src-tauri/src/db/README.md` 等 | 提及旧剪贴板语义处 | 如有，统一为新口径（执行时 grep 确认） |
| 项目记忆 `feedback_clipboard.md` + `MEMORY.md` | 整条 | 删除旧决策条目与索引（不改写为新版，避免与新 `feedback` 混淆；如需记录新决策，另建条目） |

---

## 6. 接口契约定义

- IPC 命令名与入参保持兼容：
  - `translate_capture_selection(selection_key: String) -> CapturedSelectionDto`
  - `translate_copy_result(text: String) -> Result<(), AppError>`（用户主动，不变）
  - `translate_popup_show(x, y, compact)` / `translate_popup_hide`（不变）
- `CapturedSelectionDto.via` 正常路径为 `"uia"`；`text` 为 UIA 只读结果（trim 口径与原实现一致）。
- 新增错误语义：`AppError::Probe("选区已失效，请重新选中文本")` 与 `AppError::Probe("当前窗口无法读取选区，请手动复制后使用")`；前端按 `kind` 给出中英双语提示，不直接展示后端中文串。
- 事件：`translate:selection:show` / `translate:selection:hide` 的 payload 形状不变，仅来源从剪贴板判定改为 UIA 判定。

---

## 7. 测试计划

### 7.1 Rust 单测（`selection.rs` 内）

1. `uia_selection_present_triggers_toolbar`：模拟 UIA 返回有选区，断言 emit 判定为 true，且 `Clipboard::get_text()` 前后值一致。
2. `uia_read_does_not_touch_clipboard`：用可注入的剪贴板桩（或 `arboard` 抽象层）断言探测链路 **0 次** `clear()` / `set_text()` / `Ctrl+C` 注入。
3. `non_text_clipboard_survives_gesture`：剪贴板预置非文本内容（图片/文件路径），跑完手势后断言仍可读（替代旧的 `previous=None → clear()` 场景）。
4. `uia_unavailable_does_not_emit_and_does_not_fallback_to_ctrl_c`：UIA 超时/无 TextPattern 时断言不弹工具条、不调用 `simulate_ctrl_c()`。
5. `selection_cache_expires`：内存槽超时后用旧 `selection_key` 调 `capture_selection` 返回可行动错误。
6. `uia_probe_within_budget`：断言 UIA 请求总耗时上界 ≤ 2 × `PROBE_INTERVAL_MS`（保留原预算纪律）。

### 7.2 前端测试

- `src/tests/components/TranslateComponents.spec.ts`：补充「点翻译走 captureSelection、点复制走 copyTranslateResult」的 emit/IPC 断言（现有仅测 emit 次数与文案）。
- 新增/扩展组件测试：UIA 不可用提示能按 `kind` 映射为中英双语文案；工具条「复制」按钮仍触发 `copyTranslateResult`。

### 7.3 真机验收清单（Windows 10/11）

- [ ] 复制一段文字到剪贴板 → 在任意应用拖选/双击多次 → `Ctrl+V` 仍是**原来那段文字**（不是最后划选的词，也没被清空）。
- [ ] 剪贴板放一张图片 → 拖选/双击 → 图片仍可粘贴。
- [ ] 拖选文字 → 工具条弹出 → 点「翻译」→ 译文正确，全程剪贴板内容不变。
- [ ] 点工具条「复制」→ 选中原文进入剪贴板，可直接 `Ctrl+V`（用户主动路径保留）。
- [ ] 在终端 / 远程桌面 / 自绘 UI 中拖选 → 不弹工具条、不改写剪贴板，日志可查 `uia_unavailable`。
- [ ] 快速切窗后拖选 → 不会把 Ctrl+C 发给错误窗口（因为根本不注入）。
- [ ] 工具条点外部收起 → 剪贴板不变。
- [ ] `cargo test` 与 `pnpm test` 全绿，clippy `-D warnings` 通过。

---

## 8. 风险与开放问题

| 风险 | 说明 | 缓解 |
| :--- | :--- | :--- |
| UIA 覆盖不足 | 终端、远程桌面、自绘 UI、部分 PDF 不支持 TextPattern | §4.4 兜底矩阵：不弹工具条 + 诊断日志；提供手动路径 |
| 点击工具条导致焦点转移、选区丢失 | 历史计划 `:197` 标注为最大风险 | 手势阶段即把文本读到**内存槽**，点击时不再依赖焦点选区 |
| UIA 调用阻塞 100ms scheduler | COM 跨线程可能慢 | 专用 STA 线程 + 硬超时（50/80ms），超时即不可用 |
| windows-rs 0.58 API 不确定 | 本文只给概念级调用链 | 执行前 `cargo check` 验证 feature 与接口；以 windows-rs 0.58 官方文档为准 |
| 旧记忆/契约未同步 | 会与未来会话冲突 | §3 明确删除记忆条目并改写契约，纳入验收 |

开放问题（执行时若遇到再定）：是否需要「手动粘贴输入框」作为 UIA 失效时的产品兜底？用户本轮只确认「需要兜底方案」，未指定形态；建议先做「不弹 + 可行动提示」，输入框另立任务。

---

## 9. 回滚方案

1. 从 `v0.1.1` tag 恢复以下文件：`src-tauri/src/probe/selection.rs`、`Cargo.toml`、`commands/translate.rs`、`window/manager.rs`、`i18n` 两份文案、`接口契约.md`。
2. 恢复项目记忆 `feedback_clipboard.md` 与 `MEMORY.md` 索引（从 git 或备份取回）。
3. 前端只需重新构建，无数据迁移。
4. 回滚后旧 BUG 复现，属预期。

---

## 10. 执行顺序与门槛

1. **先验证 UIA 可行性**：在独立分支上引入 windows-rs UIA feature，写一个最小 STA 线程 demo，确认能读到记事本/浏览器的选中文本；读不到就暂停并回报用户，不硬推。
2. 再改 `selection.rs` 主链路（删 clear/Ctrl+C）与内存槽。
3. 同步 Rust DTO / IPC 注释 / 前端文案。
4. 补单测与真机验收。
5. 最后同步契约文档与记忆（§3）。
6. 全部门禁通过后才提交；提交按「代码 / 文案 / 文档与记忆」拆成多个逻辑单元。

> 本子计划只定义「做什么、为什么、怎么验收」，不替代实现时的详细设计；任何第三方 API 名称以执行时的官方文档为准。
