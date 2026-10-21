# 对话管道子计划（C4：AI 对话与记忆检索管道）

| 项 | 内容 |
| :--- | :--- |
| 父计划 | [.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md](桌宠-任务拆分与开发顺序规划.md) §三 阶段 2 / 任务 C4、里程碑 M1 核心 |
| 规则依据 | `.codebuddy/rules/`：code-standards §4（AppError/禁吞错）、§5（边界——LLM 请求全 Rust、API Key 不落前端）、design-baseline §E（模块 4 管道）/§H、lint-rules §5；规格来源：`桌宠.md` 模块 4 + §5.2 记忆管道 |
| 范围边界 | **做**：LLM SSE 流式代理、三档路由与回落、混合检索 + Prompt 拼装、chat 命令实装（替换 B5 echo 桩）、异步沉淀（事实抽取/摘要/向量）、前端流消费 composable。**不含**：好感度结算与禁忌判定（C5/C7，本计划只暴露扩展点）、日记生成（C7）、气泡/输入框 UI（D1）、连通性测试 UI（D2） |
| 现状 | B5 echo 桩（`chat_send_stream` + `chat:stream:delta/done` 事件契约）已定；C3 三档配置 + `PROVIDER_PRESETS` 契约已定；`config.rs` 有 `EMBEDDING_DIM=1536`；`llm/` 四个文件为占位文档 |
| 状态 | 待执行（前置：B4、B5、C3） |

---

## 一、关键设计裁决（登记勘误 #8：Embedding 供应商选型）

| 项 | 裁决 | 理由 |
| :--- | :--- | :--- |
| Embedding 供应商 | 复用 **memory 档**的 OpenAI 兼容 `/v1/embeddings`；模型常量 `EMBEDDING_MODEL = "text-embedding-3-small"`（1536 维，与 `EMBEDDING_DIM` 匹配） | 文档未定 embedding 供应商而维度 1536 暗示 OpenAI 系；复用 memory 档免新增第四档配置 |
| 维度不匹配风险 | 若用户选硅基流动等 1024 维模型 → embedding 调用**显式校验返回维度**，不匹配报 `AppError::Llm` 并提示更换模型（不静默截断） | 静默截断会污染向量空间 |
| C4/C7 沉淀边界 | **C4**：对话后即时沉淀（事实抽取 + 会话摘要 + 向量写入），并搭建 distill 管道框架；**C7**：日周期任务（日记）、禁忌判定（并入 C7 决策已回写父计划）、记忆编辑向量重建——C7 复用 C4 的 distill 框架扩展 | 管道框架一次建成，两次消费 |
| 异步运行时 | 用 **Tauri 内置 `tauri::async_runtime`**（即 tokio），不另起 runtime；DB 阻塞操作经 `spawn_blocking` | 避免双 runtime 版本冲突 |
| TLS | reqwest 用 **`rustls-tls`**（不用 native-tls），与 SQLCipher vendored OpenSSL 链路解耦，避构建纠缠 | 降低链接复杂度 |
| 会话 ID | 前端 `crypto.randomUUID()` 生成，Rust 不引 uuid 依赖 | JS 内置即可 |

---

## 二、任务分解（C4.1–C4.8）

| ID | 任务 | 具体内容 | 依赖 | 验收标准 |
| :--- | :--- | :--- | :--- | :--- |
| C4.1 | 依赖与抽象层 | Cargo 增 `tokio(full)`、`reqwest(json,stream,rustls-tls)`、`eventsource-stream`、`futures`；`llm/client.rs`：trait `LlmClient`（`chat_stream(req) -> Stream<Delta>`、`embed(texts) -> Vec<Vec<f32>>`）+ `ReqwestLlmClient` 实现 + `MockLlmClient`（单测桩） | - | mock 驱动下游单测，无网络依赖 |
| C4.2 | SSE 流解析 | `client.rs`：`stream=true` 请求 → bytes → SSE event 流 → 解析 `choices[].delta.content`；处理 `[DONE]`、错误 JSON（4xx/5xx → `AppError::Llm(状态+厂商信息)`）、**连接/读取超时**、维度校验（embeddings）；累计器复用（流式循环内零分配约束仅限 PIXI 前端，Rust 侧允许分配但避免每 chunk 大克隆） | C4.1 | 解析单测（喂 SSE 文本样例断言 delta 序列） |
| C4.3 | 三档路由与回落 | `llm/router.rs`：`resolve(tier) -> LlmTarget`（内置额度模式 → `EMBEDDED_QUOTA_BASE_URL` 无 Key；BYOK → tiered 配置）；回落链：**所选档失败 → 必选档（DeepSeek/通义中已配者）→ fallback 档 → 本地预设话术**（预置少量无网安慰话术，emit 后 done）；全部尝试记录 `AppError::Llm` 链 | C3,C4.1 | mock 单测：memory 档 500 → main 档成功；全失败 → 本地话术 |
| C4.4 | 混合检索 + Prompt | `prompt.rs`：`build_system_prompt(retrieved: RetrievedContext, pet: &PetStateDto) -> String`——注入：人设（喵喵/第一人称/情绪化）、关系等级（Lv1-4 行为指令，数值禁播报）、记忆（KNN 命中 `source_text` + facts 按类目与时间近因）、勿扰/情绪基调占位（C6 接入）；检索器：KNN（`WHERE embedding MATCH ? AND k=?`）+ facts 近因查询（走 repositories）；**长对话截断**：最近 N 条消息 + 摘要头（N 常量进 config） | B4,C4.1 | prompt 纯函数单测（有/无记忆、等级注入、禁数值规则存在、截断生效） |
| C4.5 | chat 命令实装 | `commands/chat.rs` 替换 echo 桩：`chat_send_stream(session_id, text)` async 命令——① user 消息落库 ② spawn_blocking 混合检索 ③ `tauri::async_runtime::spawn` 流式循环 → emit `chat:stream:delta {sessionId, delta}` ④ 完成 → assistant 全文落库 + emit `chat:stream:done`；`chat_abort(session_id)` → 取消标志（`CancellationToken` 或 per-session `AtomicBool`，流循环检查）；错误 → `chat:stream:error {sessionId, kind, message}`；**AI 产出后冷眼三问执行点**（rule.md） | C4.2–C4.4 | 手工：真实 Key 流式输出；断网走回落链；abort 立即停 |
| C4.6 | 异步沉淀管道 | `llm/distill.rs`：done 后 `spawn`（不阻塞对话）——① memory 档抽取 facts（Prompt 要求 JSON 数组输出 + 宽松解析 + confidence 默认 0.6；解析失败静默跳过并 log）② 消息数超阈值 → 会话摘要 upsert `summaries`（session_id UNIQUE）③ 每条新 fact → `embed` → 写 `vec_memory`（rowid=fact.id，source_text=content）；失败重试一次后放弃；**管道留 C7 扩展点**（trait/枚举任务队列注释锚位） | C4.5 | mock 单测：JSON 解析（正常/脏数据/空）；手工：对话后库中出现 facts 与向量行 |
| C4.7 | 前端流消费 | `composables/useChatStream.ts` 实装（替换占位）：`listen("chat:stream:delta")` → **按帧节流**写入 `chatStore.bubbleText`（防打字机卡顿，父计划风险表）；done → `isStreaming=false` + 表情 happy（经 `setExpression`，C1 契约）；error → 提示态；`useChatStream` 暴露 `send/abort`；PetWindow 占位气泡接入 | B5,C4.5 | 手工：气泡占位逐字出现；abort 生效；表情联动 |
| C4.8 | 测试与收口 | 单测汇总（§六）+ 手工清单（§五）+ 回写父计划 C4 行 | 全部 | DoD 全勾 |

---

## 三、数据流总览

```
前端 send(text, sessionId)
  └─▶ chat_send_stream ─ user 消息落库(messages) ─ spawn_blocking 混合检索
        ├─ KNN: vec_memory (embedding[query]) ← embed(text-embedding-3-small@memory档)
        └─ facts 近因
  ─▶ prompt.rs 拼 System Prompt（人设+等级+记忆+禁数值）
  ─▶ router.resolve(main) ──失败──▶ 必选档 ──▶ fallback ──▶ 本地话术
  ─▶ client.chat_stream (SSE) ──emit──▶ 前端 useChatStream（帧节流→chatStore→气泡占位）
  done ─▶ assistant 落库 ─▶ distill spawn：facts(JSON)→embed→vec_memory / 摘要→summaries
```

隐私红线：Prompt 载荷仅含对话文本与本地记忆，**绝不**包含屏幕内容/进程窗口内容（零截图红线）；日志禁打 Key 与 Prompt 全文（只打长度与档位）。

---

## 四、文件清单（本计划允许的 diff 范围）

| 文件 | 操作 | 内容 |
| :--- | :--- | :--- |
| `src-tauri/Cargo.toml` | MODIFY | tokio/reqwest/eventsource-stream/futures |
| `src-tauri/src/config.rs` | MODIFY | `EMBEDDING_MODEL`、`MAX_CONTEXT_MESSAGES`、回落话术常量 |
| `src-tauri/src/llm/client.rs` | NEW | trait + Reqwest 实现 + SSE 解析 + Mock |
| `src-tauri/src/llm/router.rs` | NEW | 三档解析 + 回落链 |
| `src-tauri/src/llm/prompt.rs` | NEW | System Prompt 拼装（纯函数） |
| `src-tauri/src/llm/distill.rs` | NEW | 异步沉淀管道 |
| `src-tauri/src/db/repositories/{messages,summaries,facts,vec_memory}.rs` | MODIFY | 流式链路所需读写（messages 增删查、summaries upsert、facts 插入、vec_memory 写入+KNN） |
| `src-tauri/src/commands/chat.rs` | MODIFY | echo 桩 → 实装 |
| `src/api/chat.ts` / `src/composables/useChatStream.ts` | MODIFY | 事件消费 + send/abort + 节流 |
| `src/views/pet/PetWindow.vue` | MODIFY（最小） | 占位气泡接 chatStore |
| `src-tauri/tests/` | NEW 追加 | client 解析 / router 回落 / distill 解析 单测 |
| 父计划 C4 行 | MODIFY | 状态回写 |

**不动**：`affection/`（C5）、禁忌判定（C7）、`capabilities/`、UI 组件（D1）。

---

## 五、验证门禁与 DoD

- [ ] Rust 单测全绿：SSE 解析（含 [DONE]/错误帧）、router 回落链三分支、prompt 拼装四断言、distill JSON 三态、embedding 维度校验
- [ ] `cargo fmt --check` + `clippy --all-targets -- -D warnings` 零告警；`pnpm typecheck` 零错误
- [ ] 手工清单（真实 Key）：
  1. 流式对话：气泡占位逐字出现、done 后表情 happy、回复落库
  2. **次日召回**：今日告知"我爱吃香菜"→ 明日问"我爱吃什么"→ 回复命中（M1 验收锚点核心）
  3. 回落链：main 档填错 Key → 自动走必选档/fallback；全断网 → 本地话术且 done（不崩、不卡）
  4. abort：流式中点击中断 → 立即停止、部分文本不落库为完整回复
  5. 长对话：>N 条后摘要生效（summaries 出现行）、Prompt 长度受控
  6. 沉淀静默失败：memory 档 Key 错误 → 对话仍正常，facts 缺失 + 日志记录
- [ ] 红线自查：日志无 Key/Prompt 全文；Prompt 载荷无屏幕内容；前端仍零直连（`grep invoke` 仅 api/）
- [ ] 冷眼三问：① spawn 任务句柄是否管理（不泄漏）？② emit 的事件负载是否与前端 DTO 对齐？③ 三档配置读取是否每次实时（改配置即生效）？
- [ ] 父计划 C4 行回写；echo 桩移除标注清理

## 六、测试用例清单（Rust 侧，`#[cfg(test)]` / tests/）

| # | 用例 | 断言 |
| :--- | :--- | :--- |
| 1 | SSE 解析 | 正常 delta 序列、`data: [DONE]` 终止、错误 JSON → `AppError::Llm`、跨 chunk 分帧 |
| 2 | 维度校验 | embeddings 返回 1024 维 → 报错文案含"维度不匹配" |
| 3 | router 回落 | memory 失败→main 成功；main+memory 失败→fallback；全失败→本地话术（Mock 注入错误） |
| 4 | prompt 拼装 | 无记忆时含基础人设与禁数值句；有记忆时含命中文本；Lv 变化改变行为指令；超长截断保留摘要头 |
| 5 | distill 解析 | 合法 JSON 数组 / 带markdown围栏 / 空输出 / 非JSON——前三者产出 0-N facts，末者跳过不 panic |
| 6 | KNN 写读 | 插入 3 向量 → query 命中最近邻且 source_text 正确（真实迁移库） |

## 七、风险与回退

| 风险 | 对策 |
| :--- | :--- |
| SSE crate（eventsource-stream）API 与文档不符 | 实现前核对 crate 文档与示例；备选手写 SSE 分帧（按 `\n\n` 切 event，逻辑简单可控） |
| Embedding 供应商与 1536 维不匹配 | 显式维度校验报错 + 用户提示换模型；长期支持维度配置化（改 `EMBEDDING_DIM` 须重建 vec_memory——附录 A 已有路径） |
| 流式中 Tauri emit 洪泛卡顿 | Rust 侧按 delta 原样 emit、前端帧节流合并（风险表既定对策）；必要时 Rust 侧 50ms 合并窗口（预留） |
| 长对话 token 超限 | 最近 N 条 + 摘要头策略；超限错误映射为友好提示并自动缩减重试一次 |
| distill 失败静默丢记忆 | 重试一次 + log；E 联调期增加"沉淀失败"托盘提示（登记 E1 待办） |
| 内置额度端点未部署 | quota 模式请求失败即走回落链（C4.3 天然覆盖）；G3 发布说明标注 |
| Key 经 IPC 明文传输被质疑 | 本地进程 IPC，无网络暴露；全库加密落库；与 B5 口径一致 |

## 八、执行顺序

C4.1（抽象层）→ C4.2（SSE 解析+单测）→ C4.3（路由+单测）→ C4.4（检索+prompt+单测）→ **C4.6（蒸馏，先于命令实装以便 mock 全测）** → C4.5（chat 命令实装）→ C4.7（前端消费）→ 手工清单 → C4.8 DoD 收口。
