# IPC子计划（B5：命令层与前端封装）

| 项 | 内容 |
| :--- | :--- |
| 父计划 | [.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md](桌宠-任务拆分与开发顺序规划.md) §三 阶段 1 / 任务 B5、里程碑 M1 前置 |
| 规则依据 | `.codebuddy/rules/`：code-standards §4（错误处理/参数化/明文不落前端）、§5（边界）、design-baseline §H、development-guide（先测后集成）、lint-rules §5（门禁）；契约来源：父计划 §七 + 阶段0 子计划《接口契约.md》（若已产出） |
| 范围边界 | **只做 settings/pet/memory(只读)/chat(桩) 四组命令 + 前端封装**。不含：窗口/探针命令（C2）、LLM 真实管道（C4）、好感度结算（C5）、记忆编辑删除与向量重建（C7）、任何 UI（D*） |
| 现状 | `api/client.ts`（唯一 invoke 出口）+ `api/types.ts`（部分 DTO）已就位；`commands/{settings,pet,chat,memory}.rs` 为占位文档；`db/repositories/` 为空占位；`AppState { db: Mutex<Connection> }` 由 B3/B4 提供；greet 待移除（B3 已登记决策） |
| 状态 | **已完成**（2026-10-22，提交 d358643）——执行结果与实测修正见文末 |

---

## 一、范围与依赖状态说明

| 项 | 说明 |
| :--- | :--- |
| 前置 B4（数据层） | 命令层依赖 `AppState.db`（SQLCipher 连接）；**若 B3/B4 均未执行，本计划 B5.2 前需先完成 B4**（去重规则见 B4 计划 §执行说明） |
| 前置 B2（权限） | **自定义命令不经 capability 门控**（B2 勘误已登记）——本计划零 capabilities 改动 |
| chat 命令边界 | B5 只提供**echo 桩**（收到消息原样分片回吐，验证 SSE 事件链路 + 前端消费），真实 LLM 管道属 C4——桩在 C4 执行时整体替换，接口签名不变 |
| memory 命令边界 | 只读查询（facts/diaries 列表）；编辑/删除 + `vec_memory` 同步重建属 C7——命令签名本计划预留、实现返回 `AppError::Config("C7 接入")` |
| 与 C2 去重 | `api/window.ts`/`api/probe.ts` 由 C2.8 落位，本计划不建 |

---

## 二、任务分解（B5.1–B5.7）

| ID | 任务 | 具体内容 | 依赖 | 验收标准 |
| :--- | :--- | :--- | :--- | :--- |
| B5.1 | DTO 定稿（双端对齐） | `src/api/types.ts` 扩充全量 DTO（§三.1）；`db/models.rs` 落行结构体 + 命令 DTO（serde rename_all camelCase）；以契约文档为真源逐一对齐 | B4 | 双端字段名/类型/可空性一一对应，`pnpm typecheck` 过 |
| B5.2 | repositories 首批实装 | `db/repositories/{app_settings,pet_states,facts,diaries}.rs`：按表 CRUD（SQL 全参数化 `params![]`、行映射、`AppError::Db` 传播；只读表仅查询）；**禁止**业务决策与事件 emit（规范 §4.1） | B5.1 | 单测覆盖（B5.7） |
| B5.3 | settings 命令 | `commands/settings.rs`：`settings_get_typed(tier?)` → 返回 DTO 但 **Key 绝不明文回传**（`has_key: bool` + `key_fingerprint` 脱敏指纹）；`settings_save_typed(tier, payload)` → Key 写库（库已全库加密，无需二次加密）+ 明文不进前端持久层；`is_custom_quota_used` 读写 | B5.2 | 保存后 get 回包无 key 字段、指纹正确 |
| B5.4 | pet 命令 | `commands/pet.rs`：`pet_get_state()` / `pet_update_state(payload)`（pet_name/active_renderer/renderer_config；affection_points/level 只读透出，**结算逻辑属 C5** 不在此实现） | B5.2 | 前端读写 `pet_states` 生效 |
| B5.5 | memory + chat 命令 | `commands/memory.rs`：`memory_list_facts()` / `memory_list_diaries()`（只读）；`memory_update_fact`/`memory_delete_fact`/`memory_regenerate_diary` 签名预留（返回未接入错误）；`commands/chat.rs`：`chat_send_stream(text)` echo 桩（emit `chat:stream:delta`×N + `chat:stream:done`）+ `chat_abort()` 占位 | B5.2 | 前端收到 echo 流事件；memory 只读可用 |
| B5.6 | 前端封装与注册 | `api/{settings,pet,chat,memory}.ts`（经 `callCommand`，每个 await try/catch、错误经 `AppError` 呈现）；`types.ts` 全量 DTO；`lib.rs`：**移除 greet**（B3 决策）+ 注册六组新命令；`state.rs` 不动（B3 已定式） | B5.3–B5.5 | `grep invoke` 仅命中 `src/api/`；命令注册齐全 |
| B5.7 | 测试与验证 | `src-tauri/tests/repositories_it.rs`：app_settings 读写/JSON 解析、pet_states 单行 upsert、facts 分页、参数化（拼接注入负样例）；前端 `tauri dev` 手工清单（§五） | B5.6 | `cargo test` 全绿；手工清单全过 |

---

## 三、契约细节（本计划落地的命令面）

### 3.1 DTO（`types.ts` / Rust serde 对齐，camelCase）

```
// settings
interface LlmTierSettings { baseUrl: string; model: string; hasKey: boolean;
  keyFingerprint: string | null; modelCustom: string }
interface SettingsSnapshot { main: LlmTierSettings; memory: LlmTierSettings;
  fallback: LlmTierSettings; isCustomQuotaUsed: boolean }
settings_get_typed() -> SettingsSnapshot
settings_save_typed(tier, { baseUrl, model, apiKey?, modelCustom }) -> void
  // apiKey 仅在用户输入新值时携带；空/缺省 = 不改动存量 Key

// pet
interface PetStateDto { petName: string; relationshipLevel: number;
  affectionPoints: number; lastInteraction: string | null }
pet_get_state() -> PetStateDto
pet_update_state({ petName?, activeRenderer?, rendererConfig? }) -> PetStateDto

// memory（只读）
interface FactDto { id: number; category: number; content: string;
  confidence: number; createdAt: string }
interface DiaryDto { id: number; diaryDate: string; content: string; mood: number | null }
memory_list_facts() -> FactDto[]        // C7 再加分页
memory_list_diaries() -> DiaryDto[]

// chat（echo 桩，C4 替换实现）
chat_send_stream(sessionId: string, text: string) -> void
  // 事件：chat:stream:delta {sessionId, delta} × N → chat:stream:done {sessionId}
chat_abort(sessionId: string) -> void
```

### 3.2 Key 脱敏规则（code-standards §5 / 设计方案「Key 脱敏指纹回显」）

- `has_key`：库中存在非空 Key。
- `key_fingerprint`：Key **末 4 位**（与设计方案口径一致；若评审要求更强脱敏，备选 SHA-256 前 8 位 + `sha2` 依赖，登记决策即可）。
- 明文 Key 只允许出现在：前端输入框内存 → IPC 参数 → Rust 落库。禁止：localStorage、store、日志、错误信息、回传 DTO。

### 3.3 并发与错误

- 命令内 `state.db.lock()`：**单次获取、用完即释**，禁止嵌套锁（防死锁）；MVP 查询为短事务，同步锁可接受，C4 引入 tokio 后重查询走 `spawn_blocking`。
- 仓储错误一律 `AppError::Db`；命令层不做业务 catch——错误原样上抛由前端 `callCommand` 规整呈现（不吞错误，code-standards §4）。

---

## 四、文件清单（本计划允许的 diff 范围）

| 文件 | 操作 | 内容 |
| :--- | :--- | :--- |
| `src/api/types.ts` | MODIFY | 全量 DTO（§三.1） |
| `src/api/{settings,pet,chat,memory}.ts` | NEW | 四组封装（经 `callCommand`，零裸 invoke） |
| `src-tauri/src/db/models.rs` | MODIFY（占位→实装） | 行结构体 + 命令 DTO（serde） |
| `src-tauri/src/db/repositories/{app_settings,pet_states,facts,diaries}.rs` | NEW | 参数化 SQL 数据访问 |
| `src-tauri/src/commands/{settings,pet,chat,memory}.rs` | MODIFY（占位→实装） | 薄层命令（§二 B5.3–B5.5） |
| `src-tauri/src/lib.rs` | MODIFY | 移除 greet、注册新命令 |
| `src-tauri/tests/repositories_it.rs` | NEW | 仓储单测 |
| `.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md` | MODIFY | B5 行回写 |

**不动**：`capabilities/`（自定义命令免授权）、`state.rs`/`error.rs`/`config.rs`（B3 已定式）、`db/migrations*`（B4）、前端 UI 组件（D 阶段）、`api/{window,probe}.ts`（C2）。

---

## 五、验证门禁与 DoD

- [ ] `cargo test` 全绿（repositories_it：参数化读写、JSON 解析、单行 upsert 幂等）
- [ ] `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` 零告警；`pnpm typecheck` 零错误
- [ ] 手工清单（`pnpm tauri dev`）：
  1. 设置窗 LlmPanel（现占位）经临时调用读取 `SettingsSnapshot` → 保存带 Key → 重启后 `has_key=true`、指纹一致、**回包无明文**
  2. `pet_update_state` 改 pet_name → 重启持久；`petStore` 同步
  3. `memory_list_facts`（M1 种子后为空数组）/`memory_list_diaries` 正常返回空集不报错
  4. `chat_send_stream("hi")` → 依次收到 3 片 `chat:stream:delta` + `chat:stream:done`（echo 桩链路通）
  5. 断开 DB（改错路径模拟）→ 命令返回 `AppError::Db`，前端呈现错误不崩溃
- [ ] 红线自查：`grep -r "api_key"` 回包结构无明文字段；`grep invoke` 仅 `src/api/`；SQL 无字符串拼接（`grep format!` 于 repositories/ 为零）
- [ ] 冷眼三问：① 每个 await 是否 try/catch 且错误可见？② Key 是否有任何日志/DTO 泄漏路径？③ 锁是否单次获取即释（无嵌套）？
- [ ] greet 已移除；echo 桩标注 `[C4 替换]`；memory 写命令标注 `[C7 接入]`
- [ ] 完成后回写父计划 B5 行状态（M1 前置条件 B4/B5 达成）

## 六、风险与回退

| 风险 | 对策 |
| :--- | :--- |
| Mutex 锁跨 await/嵌套导致死锁 | 命令体结构约定：lock → 提取数据 → drop → 组装返回；clippy `await_holding_lock`（默认含）兜底 |
| `app_settings.value` JSON 双端漂移 | value 以 `serde_json::Value` 读取 → 强类型 DTO 校验序列化；契约文档为唯一字段真源 |
| Key 指纹方案争议（末4位 vs SHA-256） | 按设计方案口径末 4 位实现，评审可一键切换（集中在 settings.rs 单函数） |
| echo 桩被误当正式能力 | 桩内注释 + 事件名与 C4 完全一致（前端零改动替换），父计划 B5 行标注桩状态 |
| repositories 首批 SQL 与迁移表结构漂移 | B5.2 实现前对照附录 A §A.3 DDL；单测建表走 B4 迁移链（真实 schema） |

## 七、执行顺序

B5.1（DTO 双端）→ B5.2（repositories）→ **B5.7 前置：repositories_it 先行跑通**（rule.md 先测后集成）→ B5.3/B5.4/B5.5（命令）→ B5.6（前端封装与注册）→ 手工清单 → DoD 收口。

---

## 执行结果与计划对应情况（2026-10-22，提交 d358643）

| 计划任务 | 实际结果 | 对应 |
| :--- | :--- | :--- |
| B5.1 DTO 双端 | `db/models.rs`：行结构体 + 存储结构（三档/勿扰/旧格式）+ 出参 DTO + 入参 DTO，全部 `rename_all camelCase`；`src/api/types.ts` 增 `FactDto`/`DiaryDto`/`SaveTierPayload`/`UpdatePetPayload` | ✅ |
| B5.2 repositories | `repositories/{app_settings,pet_states,facts,diaries}.rs`：全参数化 SQL；app_settings 支持 upsert 与额度模式、pet_states 单行兜底插入与字段级更新、facts/diaries 带 limit 列表 | ✅ |
| B5.7 前置测试 | `repositories_it.rs` 6 用例**先跑通**（读写/额度/单行幂等/列表 limit/**注入负样例**/schema 漂移防护），随后才实装命令——先测后集成落地 | ✅ 顺序符合 |
| B5.3 settings | `settings_get_typed`（脱敏：hasKey + 末 4 位指纹）、`settings_save_typed`（Key 空串不改存量）；**双格式兼容**（三档优先、回落 M1 扁平种子） | ✅ |
| B5.4 pet | `pet_get_state` / `pet_update_state`（好感度只读透出，结算留 C5） | ✅ |
| B5.5 memory/chat | memory 只读两项可用；三项写命令签名预留并返回 `[C7 接入]`；chat echo 桩（3 片 delta + done，UTF-8 安全分片）+ `chat_abort` 占位，标 `[C4 替换]` | ✅ |
| B5.6 封装与注册 | `api/{settings,pet,chat,memory}.ts` 全经 `callCommand`；lib.rs 注册 11 个命令并**移除 greet**（B3 决策落地） | ✅ |

### 实测修正（三处，均为测试/工具暴露的真实问题）

1. **排序不确定**：同秒创建的 facts 因 `created_at` 相同导致 `ORDER BY` 结果不稳定 → 仓储加 `id DESC` 次级排序（生产健壮性 + 测试确定性）
2. **旧格式被误判**（单测发现）：`TieredValue` 各字段 `#[serde(default)]`，旧扁平 JSON 会"解析成功"成三档全空 → 改为**先判结构（是否含 main/memory/fallback 键）再解析**；勿扰同理（是否含 dnd 键）
3. **clippy 拦截**：手写向上取整 → `div_ceil`；嵌套 if → `match` 守卫

### 门禁终态

`cargo clippy -D warnings` ✅ / `cargo fmt --check` ✅ / **cargo test 29 passed** ✅ / `pnpm typecheck` ✅ / `pnpm build` ✅ / `pnpm test` 9 passed ✅ / `pnpm lint` 0 problems ✅ / `pnpm format:check` ✅ / husky 钩子链真实通过 ✅

### 红线自查（DoD grep 项）

| 项 | 结果 |
| :--- | :--- |
| `@tauri-apps/api/core`（invoke）命中范围 | **仅 `src/api/client.ts`** ✅ |
| `repositories/` 内拼接宏 | **零命中**（注释措辞已调整以免自查误报）✅ |
| 明文 Key 出参 | 出参仅 `hasKey`/`keyFingerprint`；`apiKey` 只存在于**入参** DTO ✅（有单测断言序列化后不含明文） |
| greet | 已移除 ✅ |

### 待用户手工验证（GUI，`pnpm tauri dev`）

1. 保存带 Key 的配置 → 重启后 `hasKey=true`、指纹一致、**回包无明文**
2. `pet_update_state` 改 pet_name → 重启持久
3. `memory_list_facts` / `memory_list_diaries` 返回空集不报错
4. `chat_send_stream("hi")` → 依次收到 3 片 `chat:stream:delta` + `chat:stream:done`（echo 桩链路通）
5. DB 故障模拟 → 命令返回 `AppError::Db`，前端呈现错误不崩溃

### 边界移交

- **C4**：替换 `chat.rs` echo 桩（签名/事件名不变，前端零改动）
- **C7**：接入 `memory_update_fact`/`memory_delete_fact`/`memory_regenerate_diary`（需同事务向量同步）
- **C2/C6**：`probe.rs` 命令（探针快照、`proactive_generate`）
- **C5**：好感度结算命令 `pet_settle_affection`（当前 pet 命令只透出、不结算）
