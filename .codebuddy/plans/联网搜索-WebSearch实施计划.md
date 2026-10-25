# 联网搜索（WebSearch）实施计划

> 子计划：为「七七」对话功能新增「联网搜索」能力（应用侧检索增强路线）。
> 生成日期：2026-10-25。状态：执行中。

## 一、背景与决策

- 应用：Tauri（Rust 后端 + Vue3 前端）桌面宠物「七七」。
- 痛点：当前对话仅依赖本地记忆 + 模型训练知识，无法回答实时/时效性问题。
- 已确认决策：
  1. **应用侧检索增强**：Rust 调搜索 API 取结果注入 system prompt，所有 OpenAI 兼容模型通用（含内置免费 GLM-4-flash），不改流式循环。
  2. **搜索服务 BYOK**：设置新增「搜索服务」配置（provider + Key + 开关），与现有平台 Key 同源、SQLCipher 加密。
  3. **聊天框手动开关**：输入区新增「联网搜索」切换，默认关。
  4. **成本自然计入 + 展示来源**：注入的额外 prompt token 随对话 usage 自然进内置额度（scene=chat）；气泡底部展示引用来源（标题+链接）。搜索 API 自身消耗走用户自填 Key，不计内置额度。

## 二、架构

```
ChatInput 开关 webSearch
  → chat_send_stream(web_search=true)
    → 已配置搜索 Key? 否 → 普通对话
        是 → web_search() 调搜索 API（失败/超时回落普通对话）
              → SearchResult 列表
              → build_chat_request 注入 system「联网检索到的资料」段
    → run_chat_task 流式
        emit chat:stream:delta
        emit chat:stream:sources {sessionId, sources}
        emit chat:stream:done
    → 前端气泡底部渲染来源
```

关键技术决策：
- 新增 `SearchProvider` trait + `web_search()`：provider 可替换（首期 Tavily，预留 Brave/SerpAPI），便于单测用桩替换。
- 不改流式循环：搜索在发请求前一次性完成，结果注入 system 文本，模型无 tool call 往返。
- 额度零改造：注入内容进入 `messages`，模型返回 `usage` 自然覆盖该部分 token；`record_call` 既有逻辑已按 scene=chat 记账 `builtin` 段。
- 配置薄壳命令：`search_settings_get` / `search_settings_save` 仿 `settings_*` 命令（薄壳 + inner 可测），Key 仅入参→落库，接口不回明文。

## 三、目录结构与改动

```
src-tauri/src/
  db/migrations/M9__search_settings.sql        # [NEW] search_settings 表
  db/repositories/search_settings.rs            # [NEW] 读写（薄壳+inner，fresh_db 单测）
  db/repositories/mod.rs                        # [MODIFY] 注册 search_settings
  commands/search_settings.rs                   # [NEW] search_settings_get / search_settings_save
  commands/mod.rs                               # [MODIFY] 注册新命令
  lib.rs                                        # [MODIFY] invoke_handler 注册 + 迁移 TARGET_VERSION=9
  llm/web_search.rs                             # [NEW] SearchResult / SearchProvider trait / Tavily 实现 / web_search()
  llm/mod.rs                                    # [MODIFY] 注册 web_search 模块
  llm/prompt.rs                                 # [MODIFY] build_system_prompt 增加 web_results 注入段
  commands/chat.rs                              # [MODIFY] chat_send_stream 加 web_search 参数 + 搜索调用 + emit sources
  config.rs                                     # [MODIFY] 搜索默认上限常量（SEARCH_MAX_RESULTS 等）
src/
  api/types.ts                                  # [MODIFY] SearchSettingsDto / ChatSource
  api/settings.ts                               # [MODIFY] getSearchSettings / saveSearchSettings
  api/chat.ts                                   # [MODIFY] chatSendStream 增加 webSearch 参数
  components/pet/ChatInput.vue                  # [MODIFY] 新增「联网搜索」开关
  views/chat/* 对话面板                         # [MODIFY] 传递开关状态到发送
  components/pet/BubbleMessage.vue              # [MODIFY] 底部渲染引用来源
  state/chatStore.ts                            # [MODIFY] 监听 chat:stream:sources 写入消息
  tests/components/ChatInput.spec.ts            # [NEW] 开关行为
docs/design/接口契约.md                          # [MODIFY] 增 search_settings_* 与 chat 新增参数/事件
```

## 四、实现要点

- `chat_send_stream` 新增参数 `web_search: bool`；命令层读 `search_settings` 决定是否发起搜索。
- 注入位置：`build_system_prompt` 增加 `web_results: Option<&[SearchResult]>` 参数，纯函数、单测覆盖。
- 事件：`run_chat_task` 在 `done` 前 emit `chat:stream:sources {sessionId, sources}`，前端 chat store 收集到对应 message。
- 单次搜索最多 5 条、每条 snippet 截断 400 字，注入上限约 2k 字；超时 10s 连接 / 30s 总；失败仅 log + 回落普通对话。

## 五、设计风格

延续 Jelly-Toy 治愈系：圆角柔和、粉彩玻璃拟态、微动效。聊天框「联网搜索」胶囊开关（🌐，默认关，开启态 peach 描边点亮）；气泡底部来源 chip（标题+外链，--bg-deep 底、--ink-soft 文字、圆角 10px、hover 微上移）。

## 六、验收

- 配置搜索 Key 后开启开关 → 实时资料注入、气泡展示来源。
- 未配置 Key / 搜索失败 → 自动回落普通对话，不报错不打断。
- 内置额度随用量自然增长（scene=chat）。
- 全量门禁：clippy 0 / cargo test / vue-tsc / eslint / prettier / vitest 通过。
- 单独提交（fix/feat: 联网搜索）。
