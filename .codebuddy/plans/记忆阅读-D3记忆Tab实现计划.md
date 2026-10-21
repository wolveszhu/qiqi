# 记忆阅读子计划（D3：MemoryPanel 记忆阅读区 Tab）

| 项 | 内容 |
| :--- | :--- |
| 父计划 | [.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md](桌宠-任务拆分与开发顺序规划.md) §三 阶段 3 / 任务 D3、里程碑 M3 |
| 规则依据 | `.codebuddy/rules/`：code-standards §7（组件约定/no-v-html）、design-baseline §F（模块 8"用户可修正记忆"）；实现蓝图：[`docs/design/窗口B-设置界面设计方案.md`](../../docs/design/窗口B-设置界面设计方案.md) §4.5（MemoryPanel 规格）+ §5.2（DiaryCard/FactChip 组件）+ §6（破坏性操作二次确认）；**选型关联**：日记/蒸馏/重建均走 memory 档（父计划选型方案：qwen-flash 等低成本模型），本计划无 LLM 配置交互、仅消费 C7 结果——无需对齐改动 |
| 范围边界 | **做**：日记列表与日期导航、facts 标签云（分组/行内编辑/删除/新增）、重新生成。**不含**：日记生成与向量同步逻辑（C7 已备，本计划仅消费）、托盘/铃铛（D4） |
| 现状 | D2 后：MemoryPanel 为空态骨架（`memory_list_diaries` 空态已接）；C7 已实装 `memory_update_fact`/`memory_delete_fact`/`memory_regenerate_diary`/`memory_list_facts`/`memory_list_diaries`；`components/memory/` 占位 |
| 状态 | 待执行（前置：C7、D2）——小体量任务 |

---

## 一、关键裁决（登记勘误 #14：新增记忆缺口）

| # | 问题 | 裁决 |
| :--- | :--- | :--- |
| 1 | **用户无法新增记忆**：`facts` 仅由 LLM 蒸馏产生，C7 只做了编辑/删除——"用户可修正记忆"缺少正向入口 | 本计划增 `memory_add_fact(content, category)` 命令：insert facts + embed + `vec_memory` **同事务**（复用 C7.3 编辑的同步模式）；UI 入口 = 标签云"＋ 添加记忆"按钮（默认 category=preference，可切） |
| 2 | mood 数值 → 展示映射 | `constants/mood.ts`：mood → { emoji, label }（如 1=😔低落 / 2=😌平静 / 3=😺开心 / 4=🥰雀跃，具体映射 E3 打磨可调）；LLM 输出 mood 超范围 → 回退"平静" |
| 3 | 日期导航策略 | 列表按 `diary_date` 倒序 + **顶部日期锚点条**（有日记的日期 chip，点选滚动定位）；默认定位今日（无今日则最近一篇）；不做日历控件（设计方案 §4.5 无此要求） |
| 4 | 重新生成确认 | 触发前二次确认（覆盖当日日记不可撤销，设计方案 §6）→ loading 态（禁重复点击）→ 完成刷新 + toast |
| 5 | 流式/长日记渲染 | 日记内容纯文本插值渲染 + `white-space: pre-wrap`；超长卡片内滚动（max-height），**禁 v-html** |

---

## 二、任务分解（D3.1–D3.5）

| ID | 任务 | 具体内容 | 依赖 | 验收标准 |
| :--- | :--- | :--- | :--- | :--- |
| D3.1 | 新增记忆命令 | `commands/memory.rs` 增 `memory_add_fact(content, category)`：校验（非空、长度 ≤200）→ 事务内 insert facts + embed + 写 `vec_memory`（rowid=fact.id，同 C7.3 模式）；embed 失败回滚返回 `AppError::Llm`；`repositories/{facts,vec_memory}.rs` 增 insert 路径；单测：新增后 KNN 命中 | C7 | 命令可用，向量一致 |
| D3.2 | 日记列表与导航 | `components/memory/DiaryCard.vue`：日期 + mood 徽标（constants/mood.ts 映射）+ 内容（pre-wrap、max-height 内滚）+ "重新生成"按钮；MemoryPanel：倒序列表 + 日期锚点条（点选滚动）+ 今日定位 + 空态（"今天还没有日记，和我说说话吧"） | D2 | 导航/定位/空态符合 §4.5 |
| D3.3 | 标签云与记忆编辑 | `components/memory/FactChip.vue`：三组 category 分组（偏好/事件/画像，中文标题）→ chip 行内编辑（点击进 input，Enter 确认 / Esc 取消）+ 删除（二次确认 `memory_delete_fact`）；"＋ 添加记忆"（D3.1）；编辑/删除/新增后**乐观更新 + 失败回滚** | D3.1 | 增删改全链路生效且向量同步（C7 验收延续） |
| D3.4 | 重新生成流程 | 生成中 loading（卡片骨架 + 按钮禁用防重复）→ 完成刷新 + AppToast；确认弹层文案明确"将覆盖 X月X日 日记" | D3.2 | 覆盖语义正确、无并发双击 |
| D3.5 | 走查与收口 | 设计方案 §4.5 走查 + 手工清单（§四）+ 父计划 D3 行回写（M3 验收：设置窗可见日记与记忆标签、可修正记忆并同步向量） | 全部 | DoD 全勾 |

---

## 三、交互细节（依设计方案 §4.5/§6）

```
MemoryPanel
├── 日记区（上部，占 55%）
│   ├── 日期锚点条：[10-21][10-20][10-18]…（chip，今日高亮）
│   └── DiaryCard：😺开心 · 10月21日 | 正文(pre-wrap) | [重新生成]
├── 分割线（纸纹）
└── 记忆标签云（下部，占 45%）
    ├── 你喜欢 / 发生过 / 关于你（三组标题）
    ├── FactChip：内容 · [编辑✎][删除🗑]（hover 显操作）
    └── ＋ 添加记忆（行内 input + 类目选择）
```

- 所有破坏性操作（删除/重新生成）二次确认；行内编辑取消不落库。
- 空态文案带引导（符合"零门槛"设计定位）。
- 列表数据 `memory_list_diaries`/`memory_list_facts` 启动加载一次 + 操作后**局部刷新**（不整面板重载）。

---

## 四、文件清单（本计划允许的 diff 范围）

| 文件 | 操作 | 内容 |
| :--- | :--- | :--- |
| `src/components/memory/{DiaryCard,FactChip}.vue` | NEW | 组件实装 |
| `src/views/settings/panels/MemoryPanel.vue` | MODIFY | 占位 → 实装 |
| `src/constants/mood.ts` | NEW | mood 映射（E3 可调） |
| `src/api/memory.ts` | MODIFY | addFact 封装 |
| `src-tauri/src/commands/memory.rs` | MODIFY | `memory_add_fact` |
| `src-tauri/src/db/repositories/{facts,vec_memory}.rs` | MODIFY | insert + 同步路径 |
| `src-tauri/tests/` | NEW 追加 | add_fact 向量一致性单测 |
| 父计划 D3 行 / 契约文档 | MODIFY | 状态回写 + 命令清单补录 |

**不动**：C7 蒸馏/日记生成逻辑、其他面板、`capabilities/`（自定义命令免授权）。

---

## 五、验证门禁与 DoD

- [ ] `cargo test` 全绿（add_fact 后 KNN 命中、脏输入拒绝）；`pnpm typecheck` 零错误
- [ ] 手工清单：
  1. 无日记时空态引导；生成今日日记（经 D3.4 或 C7 定时）→ 卡片出现、mood 徽标正确
  2. 日期锚点切换历史日记；重新生成 → 确认弹层 → 覆盖成功 + toast
  3. 编辑某条记忆 → 保存后看板即时更新；随后对话验证 AI 用新内容（向量已同步）
  4. 删除记忆 → 二次确认 → 消失；AI 不再引用
  5. ＋新增"我喜欢吃香菜"（preference）→ 次日对话"我爱吃什么"命中（M1 召回锚点闭环）
  6. 空/超长输入（>200 字）被拒绝并提示
- [ ] 红线自查：无 `v-html`；破坏性操作均有二次确认；乐观更新失败有回滚
- [ ] 冷眼三问：① add_fact 与蒸馏并发写 facts 是否安全（同锁串行）？② 行内编辑取消是否零副作用？③ 事件/DTO 与 C7 契约对齐？
- [ ] 父计划 D3 行回写（M3 验收达成）

## 六、风险与回退

| 风险 | 对策 |
| :--- | :--- |
| add_fact embed 失败率高（用户手输内容不规范） | 失败返回明确错误（不静默），内容保留在输入框供重试 |
| 大量 facts 标签云溢出 | 三组折叠（默认展开偏好组）+ 分组内 max-height 滚动 |
| 日期锚点多（一年 365 chip） | 锚点条横向滚动 + 仅显示近 30 天，更早经"加载更早"追加 |
| mood 映射与 LLM 输出值域不一致 | 契约固定 mood ∈ 0-4（0=未记录），越界回退；映射表单点维护 |

## 七、执行顺序

D3.1（新增记忆命令+单测）→ D3.2（日记列表）→ D3.3（标签云）→ D3.4（重新生成流程）→ 手工清单 → D3.5 DoD 收口。
