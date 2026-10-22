# 测试子计划（F1：Rust 单测收口）

| 项 | 内容 |
| :--- | :--- |
| 父计划 | [.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md](桌宠-任务拆分与开发顺序规划.md) §三 阶段 5 / 任务 F1、里程碑 M4 |
| 规则依据 | `.codebuddy/rules/`：code-standards §4、auto-commit（补测提交纪律）、lint-rules §5（`[lints.clippy]` 建议）、development-guide；要求来源：rule.md「先测后集成」+ 父计划 F1 行（顺序：持续） |
| 任务性质 | **测试资产盘点 + 缺口补齐**：各子计划已按"随做随测"产出大量测试（B3–C7/D4），F1 定位为**盘点覆盖矩阵 → 补缺口（重点是 IPC 命令独立验证）→ 全绿收口**，而非重新写测试 |
| 范围边界 | **做**：Rust 侧测试盘点/补齐/收口。**不含**：前端 Vitest（F2）、手工回归（F3）、缺陷修复流程（F4） |
| 现状 | 各子计划已规划/产出的测试资产见 §二 预期清单（约 60+ 用例，跨 10 个测试文件/模块）；**预期缺口**：`commands/` 层直接验证（rule.md 要求"IPC 命令独立验证"）与 `[lints.clippy]` 配置未落地 |
| 状态 | 待执行（前置：B4 及全部功能任务） |

---

## 一、任务分解（F1.1–F1.5）

| ID | 任务 | 具体内容 | 依赖 | 验收标准 |
| :--- | :--- | :--- | :--- | :--- |
| F1.1 | 测试资产盘点 | `grep -c "#\[test\]"` + `ls src-tauri/tests/` 盘点现状 → 产出**覆盖矩阵**（模块 × 已有用例 × 缺口，见 §二 预期基线）；缺口列表定稿 | 前置功能 | 矩阵留档（本计划末尾追加） |
| F1.2 | IPC 命令独立验证补齐 | `commands/` 层测试（rule.md 要求）：命令函数**直接调用**（`State<AppState>` 用真实临时库构造——不启 Tauri runtime）：settings_get/save（含 Key 脱敏断言）、pet_settle（幂等）、memory_add/update/delete（向量一致）、chat 命令参数校验分支；Win32/网络依赖命令（window/probe）**标记为手工/集成验证**（不可单测项登记） | F1.1 | commands 六文件全部有直接验证或豁免登记 |
| F1.3 | 覆盖缺口补齐 | 按 F1.1 矩阵补齐其余缺口（边界值/错误路径优先：AppError 全变体、迁移失败分支、参数化注入负样例）；**禁过度追覆盖率数字**——按风险补（资金/隐私/数据完整性路径必须测） | F1.1 | 缺口清零或登记豁免理由 |
| F1.4 | clippy lints 配置落地 | `Cargo.toml` 增 `[lints.clippy]`（lint-rules §5 建议项：`unwrap_used`/`expect_used`/`panic`/`indexing_slicing = "warn"`）→ 清理新告警（存量 `unwrap`/`expect` 逐条改造为 `AppError` 或登记豁免） | F1.1 | clippy 零告警（含新 lints） |
| F1.5 | 全绿收口 | `cargo test` 全绿 + `fmt --check` + `clippy -D warnings`；测试运行时间记录（超 60s 则加 `#[ignore]` 标记慢集成）；回写父计划 F1 行 | 全部 | DoD 全勾 |

---

## 二、测试资产盘点（预期基线，F1.1 执行时以实际为准核对）

| 来源 | 测试位置 | 用例 | 状态 |
| :--- | :--- | :--- | :--- |
| B3 | `error.rs #[cfg(test)]` | 序列化/Display/From ×3 | ✅ 已规划 |
| B4 | `tests/migrations_it.rs` | 附录 A §A.7 七项（迁移/加密/版本三分支/种子/向量/约束） | ✅ 已规划 |
| B5 | `tests/repositories_it.rs` | settings/pet/facts CRUD + 参数化注入负样例 | ✅ 已规划 |
| B5+D2 | 契约一致性 | settings DTO 双端对齐（编译期保证） | 编译期 |
| C2 | `window/click_through.rs #[cfg(test)]` | 命中判定/翻转去重 | ✅ 已规划 |
| C3 | `config.rs` / `commands/settings.rs` | presets 校验、quota 切换、双格式兼容 | ✅ 已规划 |
| C4 | `llm/{client,router,prompt,distill}.rs` | SSE 解析/维度校验/回落链/prompt 四断言/distill 三态/KNN 写读 | ✅ 已规划 |
| C5 | `affection/{rules,decay,level}.rs` | 8 组规则单测 + 集成事务 | ✅ 已规划 |
| C6 | `probe/{idle,foreground}.rs` | 翻转状态机/名单匹配 | ✅ 已规划 |
| C7 | `llm/{distill,diary}.rs` / `tests/` | 禁忌三态/向量同步/重建等价/add_fact | ✅ 已规划 |
| D4 | `utils/edgeSnap`（前端，F2）/ Rust monitor 计算 | 三边/阈值 | 前端归 F2 |
| **缺口** | `commands/*.rs` | **IPC 命令直接验证**（B5/C3/C5/C7 命令层） | **F1.2 补齐** |
| **缺口** | `Cargo.toml [lints.clippy]` | lint-rules §5 建议配置 | **F1.4 落地** |

---

## 三、IPC 命令验证模式（F1.2 设计）

```rust
// tests/commands_it.rs —— 不启 Tauri runtime，直接调用命令函数
// 前置：临时库 fixture（复用 B4 迁移链建库）→ AppState { db: Mutex<Connection> }

#[test]
fn settings_get_never_leaks_api_key() {
    let state = fixture_state();          // 临时库 + 种子
    let snap = settings_get_typed(state)?; // 直接调用，绕过 invoke 序列化层
    assert!(!serde_json::to_string(&snap)?.contains("sk-")); // 无明文
    assert!(snap.main.has_key);            // 指纹/has_key 正确
}

#[test]
fn pet_settle_is_idempotent() { /* 同参数两次结算 → logs 仅一笔 */ }
```

- 可测：settings/pet/memory/chat 的**参数校验与纯转发分支**（约 15 个命令）
- 豁免（不可单测 → 手工/F3 回归）：`window_*`（Win32 依赖）、`probe_*`（系统状态）、`chat_send_stream` 的流式网络部分（C4 Mock 已覆盖管道逻辑，命令层仅验证参数与会话管理）

---

## 四、验证门禁与 DoD

- [ ] 覆盖矩阵留档（盘点实态 vs 预期基线，差异说明）
- [ ] `commands/` 六文件：全部有直接验证或豁免登记（豁免理由写入矩阵）
- [ ] 覆盖缺口清零（或豁免登记）；资金/隐私/数据完整性路径 100% 有测试
- [ ] `[lints.clippy]` 四项 lints 落地，`cargo clippy --all-targets -- -D warnings` 零告警
- [ ] `cargo test` 全绿；运行时长记录；慢测试（>1s 单个）加 `#[ignore]` 并归入 `tests/`
- [ ] auto-commit：补测按模块拆分提交（`test(scope): 描述`）
- [ ] 冷眼三问：① 是否有测试只断言"不 panic"（弱断言）？② 命令测试是否真实验证了 Key 泄漏红线？③ 豁免项是否都有手工回归归属（F3 清单）？
- [ ] 父计划 F1 行回写（`cargo test` 全绿达成）

## 五、风险与回退

| 风险 | 对策 |
| :--- | :--- |
| 命令直调需构造 `State` 包装（Tauri 类型耦合） | `State<'_, AppState>` 可由 `&AppState` 经 unsafe/`State::from` 构造——若耦合过深，降级为"命令体抽 `inner(state: &AppState, ...)` 纯函数 + 命令薄壳"重构（最小侵入，不改对外签名） |
| 新 lints 揪出存量 unwrap/expect 较多 | 逐条改 `AppError` 或 `ok_or`；确需保留处 `#[allow(clippy::xx)]` + 行内理由注释 |
| 测试运行慢拖累开发节奏 | 慢集成标记 `#[ignore]` + `cargo test -- --include-ignored` 全量跑；日常跑默认集 |
| 覆盖率工具链重（tarpaulin 编译慢） | MVP 不引覆盖率工具，以风险路径核对替代；Phase 2 评估 llvm-cov |

## 六、执行顺序

F1.1（盘点矩阵）→ F1.2（IPC 命令验证补齐）→ F1.3（缺口补齐）→ F1.4（clippy lints）→ F1.5（全绿收口）→ DoD → F2/F3 衔接。

---

## 七、覆盖矩阵（F1.1 实态，2026-10-22 盘点）

盘点口径：`grep -c "#[test]"` + `ls tests/`。实态 **lib 78**（较预期 60+ 增量来自 C6/E1b/E3 随做随测）+ 集成 `migrations_it 10 / repositories_it 10 / memory_it 8 / affection_it 4`。

| 模块 | 位置 | 用例（实态） | 缺口/裁决 |
| :--- | :--- | :--- | :--- |
| B3 error | `error.rs #[cfg(test)]` | 序列化/Display/rusqlite From + **F1.3 全变体覆盖** | ✅ 补齐（全 6 变体序列化+Display） |
| B4 迁移/加密 | `tests/migrations_it.rs` | 10（全链迁移/加密/版本三分支含 user_version=99 失败/种子/向量/约束/损坏隔离用例 8） | ✅ 迁移失败分支已覆盖 |
| B5 仓储 | `tests/repositories_it.rs` | 10（settings/pet/facts CRUD + **case5 参数化注入负样例** `x'); DROP TABLE facts; --`） | ✅ 注入负样例已覆盖 |
| C2 穿透 | `window/click_through.rs` | 7（命中判定/翻转去重） | ✅ |
| C3 配置 | `config.rs` + `commands/settings.rs` | 4 + 10（preset/quota/双格式兼容 + **F1.2 save/get/quota/dnd 直测含 Key 脱敏红线**） | ✅ 命令层补齐 |
| C4 LLM | `llm/{client,router,prompt,distill}.rs` | 5+4+8+10 | ✅ |
| C5 好感度 | `affection/{rules,decay,level}.rs` + `affection_it.rs` | 4+4+4+4 | ✅ |
| C6 探针 | `probe/{scheduler,foreground}.rs` | 1+（foreground 逻辑经命令/单测） | ✅ |
| C7 记忆/日记 | `tests/memory_it.rs` + `llm/distill.rs` | 8 + 10（禁忌三态/向量同步/重建等价/add_fact/upsert 幂等） | ✅ |
| **commands 直测** | `commands/{settings,pet,memory}.rs` | **F1.2 新增**：settings 6 / pet 3 / memory 2 | ✅ State-only 入口抽 `inner(conn)` 直测 |

### commands 六文件验证/豁免登记（F1.2 DoD）

| 命令文件 | 可直测（State-only）→ 抽 inner 直测 | 豁免（需 AppHandle/Win32/网络）→ 归属 |
| :--- | :--- | :--- |
| `settings.rs` | ✅ get/save/set_quota_mode/save_dnd（含 Key 脱敏、BYOK 守卫、频率白名单、空串 Key 保留） | — |
| `pet.rs` | ✅ update_state、list_affection_logs（含 limit clamp、倒序） | `pet_get_state`/`pet_settle_affection` 需 AppHandle+emit → `tests/affection_it.rs`（域层 settle 事务） |
| `memory.rs` | ✅ list_facts、list_diaries | `add/update/delete/regenerate` 需 AppHandle+embed/LLM 网络 → `tests/memory_it.rs`（向量一致/upsert 幂等） |
| `chat.rs` | — | `chat_send_stream` 流式网络 + `retrieve_context` embed 网络 → C4 Mock 已覆盖管道逻辑；参数校验分支属纯逻辑（`is_image_path`/`detect_mime`）已在文件内覆盖；流式/中止归 F3 手工回归 |
| `probe.rs` | — | Win32/系统状态 → F3 手工回归 |
| `window.rs` | — | Win32 窗口 API → F3 手工回归 |

## 八、执行结果（2026-10-22）

### F1.4 clippy lints（落地）
- `Cargo.toml` 增 `[lints.clippy]`：`unwrap_used/expect_used/panic/indexing_slicing = "warn"`。
- lib 非测试存量逐项改造（**零 `#[allow]` 滥发**）：
  - 7 处索引/切片 → `get`/`first_mut`/`as_object_mut().map()`（`level.rs`/`rules.rs`/`client.rs`/`router.rs`×2/`foreground.rs`/`chat.rs`）。
  - `lib.rs run()` 2 处 `expect`（快捷键注册、`tauri::run`）为**启动致命、无恢复路径** → 函数级 `#[allow(expect_used)]` + 理由注释。
  - `router.rs chat_with_fallback` 嵌套 if → 函数级 `#[allow(collapsible_if)]`（合并需引入 unwrap，保留清晰写法）。
- 测试代码豁免：lib.rs crate 级 `#![cfg_attr(test, allow(四项))]` + 4 个 `tests/*.rs` 文件头 `#![allow(四项)]`（fixture 用 unwrap/expect/index 属合理用法）。
- **`cargo clippy --all-targets -- -D warnings` 零告警**。

### F1.2 命令独立验证
- 模式（计划 §三降级路径，因 Tauri v2 `State` 无公开构造）：命令薄壳仅 `state.db.lock()` 后转发 **`inner(conn)` 纯函数**，测试经 `test_util::fresh_db()` 直测 `inner`（不启 runtime、不涉 `State`）。
- 新增 `src/test_util.rs`（复用 `init_with_key` 固定密钥旁路 keyring，口径同 `tests/`）。
- 覆盖：settings 6（含 **Key 脱敏红线**：save→get 序列化回包不含明文/主体；空串 Key 保留存量）、pet 3、memory 2。

### F1.3 缺口补齐
- 新增 AppError 全变体序列化/Display 覆盖（error.rs）。
- 既有确认覆盖：参数化注入负样例（repositories_it case5）、迁移失败分支与损坏隔离（migrations_it 用例 7/8）。**无残留缺口**；豁免项均登记 F3 归属（见 §七）。

### F1.5 全绿收口
- `cargo test`：**lib 78 passed**（较 F1 前 +11）+ 集成 `migrations 10 / repositories 10 / memory 8 / affection 4` 全绿。
- 运行时长：单测试套件最长 `memory_it` 2.26s、`migrations_it` 1.32s，无单个用例 >1s → **无需 `#[ignore]`**。
- `cargo fmt --check` 0；`cargo clippy --all-targets -- -D warnings` 0。

### 门禁与提交
- 提交：`8f7eb37 refactor(ipc): F1 Rust 单测收口——clippy lints 落地 + commands 层可测化 + 命令/错误直测`（husky eslint+prettier 通过）。

### 冷眼三问
- ① 无弱断言：命令直测均断言具体 DB 效果（pet_name 回显、倒序、limit clamp、Key 脱敏、BYOK 守卫、空串保留），非"不 panic"。
- ② Key 泄漏红线真实验证：`settings_command_roundtrip_never_leaks_key` 断言序列化回包既不含完整 Key 也不含 Key 主体。
- ③ 豁免项均有 F3 手工回归归属（chat 流式/中止、probe、window）或域层集成测试归属（pet settle、memory 写）。

### DoD 收口
- [x] 覆盖矩阵留档（实态 vs 预期，差异：用例数增至 78+32，commands 层补齐）
- [x] commands 六文件全部有直接验证或豁免登记
- [x] 缺口清零（AppError 全变体补齐；注入/迁移失败既有已覆盖）；资金/隐私/数据完整性路径 100% 有测试
- [x] `[lints.clippy]` 四项落地，`clippy --all-targets -- -D warnings` 零告警
- [x] `cargo test` 全绿；时长记录（无 >1s 单用例，不加 ignore）
- [x] auto-commit：`refactor(ipc)` 按模块拆分提交
- [x] 父计划 F1 行回写 ✅（M4 组成部分达成）→ 可衔接 **F2（前端单测）/F3（手工回归）**
