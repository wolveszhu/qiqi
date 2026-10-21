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
