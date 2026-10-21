# Rust骨架子计划（B3：模块骨架与 AppError 错误体系）

| 项 | 内容 |
| :--- | :--- |
| 父计划 | [.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md](桌宠-任务拆分与开发顺序规划.md) §三 阶段 1 / 任务 B3 |
| 规则依据 | `.codebuddy/rules/`：code-standards §1/§4（命名/错误处理）、design-baseline §G/§H（边界/AppError）、lint-rules §5（clippy 门禁）、《项目目录结构规范》§4（模块职责与红线） |
| 范围边界 | **只做骨架核对、收口与最小补全**。不含：DB 连接与迁移（B4）、命令实现（B5/C2）、任何业务逻辑 |
| 现状（骨架任务已提前完成大部分） | ✅ 已落地：`lib.rs` 模块声明与组装（opener 插件 + greet）、`error.rs`（AppError 六变体 Db/Llm/Window/Probe/Io/Config + serde tag/content + `From<rusqlite::Error>`）、`config.rs`（EMBEDDING_DIM/PROBE_INTERVAL_MS）、`state.rs`（占位 `AppState`）、六域 `mod.rs`（db/llm/affection/proactive/probe/window，含职责与 TODO 注释）、`commands/` 六域占位；✅ `cargo clippy --all-targets -- -D warnings` 与 `cargo fmt --check` 已全绿 |
| 剩余缺口 | ① `state.rs` 仍是占位结构（`AppState` 无字段）；② AppError 序列化无 `#[test]`（rule.md：先测后集成）；③ 骨架与规范 §4 的走查核对未留档；④ lib.rs 缺后续任务接入的注释锚位 |
| 状态 | 待执行（小体量收口任务） |

---

## 一、任务分解（B3.1–B3.5）

| ID | 任务 | 具体内容 | 依赖 | 验收标准 |
| :--- | :--- | :--- | :--- | :--- |
| B3.1 | 骨架走查核对 | 对照《目录结构规范》§4 与 code-standards 逐项走查现有骨架：模块边界红线（`grep "tauri::command"` 仅命中 `commands/`；领域模块无 I/O 越界注释与实际不符项）、常量真源（`config.rs` 单点）、命名（snake_case 模块/PascalCase 类型）；结果留档（走查清单勾选） | - | 走查零违规，或违规项当场修正 |
| B3.2 | `AppState` 正式化 | `state.rs`：占位结构 → `pub struct AppState { pub db: Mutex<Connection> }`（`db` 字段由 B4 注入连接，本任务仅定义结构签名 + doc 注明"经 `app.manage()` 注册，命令层经 `State<AppState>` 访问"）；预留后续字段注释锚位（cursor/hit_region 缓存属 C2，不提前加，防 unused） | - | 编译通过，clippy 无 unused 告警（pub 可达项不触发 dead_code） |
| B3.3 | AppError 序列化单测 | `error.rs` 增 `#[cfg(test)]`：① `serde_json::to_string` 快照断言（`Db("x")` → `{"kind":"Db","message":"x"}`，camelCase tag/content）；② `Display` 输出格式断言（`[db] x`）；③ `From<rusqlite::Error>` 映射断言——rule.md「先测后集成」的第一个落点 | - | `cargo test` 全绿 |
| B3.4 | lib.rs 组装锚位 | `lib.rs` 现有组装（插件/handler）**不重构**，仅补注释锚位，标明后续任务接入点：`// [B4] .setup(): db::connection::init + app.manage(AppState)`、`// [C2] 插件: global-shortcut/autostart + 托盘 + 探针线程`、`// [B5] invoke_handler: 追加六域命令`——保证后续 diff 最小化、组装点唯一 | B3.1 | 锚位注释与各子计划接入点一一对应 |
| B3.5 | 门禁与回写 | 复跑全套门禁；回写父计划 B3 行状态（标注"骨架已提前完成，本任务收口"） | 全部 | DoD 全勾 |

---

## 二、走查核对清单（B3.1 依据）

| # | 检查项 | 依据 | 当前状态 |
| :--- | :--- | :--- | :--- |
| 1 | `#[tauri::command]` 仅出现在 `commands/`（+lib.rs 的 greet 遗留项→登记：greet 随 B5 移除或迁入 commands） | 规范 §7 | greet 在 lib.rs（脚手架遗留），登记处理决策 |
| 2 | 领域模块（affection/proactive）无 I/O；db 仅 SQL；probe 只读系统；window 只改窗口 | 规范 §4.1 | 均为文档占位，无实现，天然合规 |
| 3 | 常量真源唯一（`config.rs`；`EMBEDDING_DIM`/`PROBE_INTERVAL_MS` 无第二定义） | 规范 §4.1 | ✅ |
| 4 | `error.rs` 六变体与前端 `types.ts` `AppErrorKind` 一致（含 Unknown 兜底差异说明） | 阶段0-A2 | ✅ 前端多 `Unknown`（非 Rust 错误兜底），契约文档登记 |
| 5 | 模块命名 snake_case、类型 PascalCase、常量 SCREAMING_SNAKE | code-standards §1 | ✅ |
| 6 | `llm/` vs code-standards §2 的 `ai/` 目录名差异 | 规范 §9 差异登记 | 按规范 v1.1 用 `llm/`，已登记 |
| 7 | 空占位模块（doc-only `mod.rs`）不触发 clippy 告警 | lint-rules §5 | ✅ 已验证全绿 |

---

## 三、文件清单（本计划允许的 diff 范围）

| 文件 | 操作 | 内容 |
| :--- | :--- | :--- |
| `src-tauri/src/state.rs` | MODIFY | AppState 正式化（B3.2） |
| `src-tauri/src/error.rs` | MODIFY（仅追加） | `#[cfg(test)]` 序列化/Display/From 单测（B3.3） |
| `src-tauri/src/lib.rs` | MODIFY（仅追加注释） | 组装锚位（B3.4）；greet 处置决策 |
| `.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md` | MODIFY | B3 行回写 |
| 走查清单 | 留档于本计划文件末尾（执行后追加勾选） | B3.1 |

**不动**：六域 `mod.rs`、`commands/*` 占位、`config.rs`、前端、Cargo.toml（零新依赖）。

---

## 四、验证门禁与 DoD

- [ ] 走查清单 7 项全过（§二）
- [ ] `cargo test` 全绿（新增 3 个 error 单测）
- [ ] `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` 零告警
- [ ] `pnpm typecheck` 不受影响（前端零改动，复验即可）
- [ ] greet 处置有明确决策：随 B5 一并移除（推荐，避免死代码与误导）或迁入 `commands/`，登记于 lib.rs 注释
- [ ] 冷眼三问：① AppState 字段是否全部有消费方或明确预留任务？② 锚位注释与各子计划接入点是否一一对应？③ 有无"顺手实现"越界到 B4/B5 范围的代码？
- [ ] 父计划 B3 行已回写（M0 验收项 `clippy` 零错保持达成）

## 五、风险与回退

| 风险 | 对策 |
| :--- | :--- |
| AppState 提前定义 `db` 字段引发 unused 告警 | pub 结构 + pub 字段在 lib crate 中为公开 API，不触发 dead_code；若所装 clippy 版本仍告警，则回退为占位结构 + doc 说明，定义移至 B4 |
| 序列化快照测试对 serde 版本敏感 | 断言用 `serde_json::Value` 逐字段比较而非整串字符串比较，降低脆性 |
| greet 遗留引发的边界争议 | 本计划仅登记决策不实现；移除动作归 B5（命令层任务），diff 责任清晰 |
| 与 B1/B4 子计划重叠（B1 也动 lib.rs） | B1 只加 `on_window_event` 接线，B3 只加注释锚位——两者无行冲突；执行顺序 B3 → B1 时锚位先行，反之锚位补写 |

## 六、执行顺序

B3.1（走查核对）→ B3.2（AppState 正式化）→ B3.3（AppError 单测）→ B3.4（组装锚位）→ B3.5（门禁回写）→ DoD 收口。
