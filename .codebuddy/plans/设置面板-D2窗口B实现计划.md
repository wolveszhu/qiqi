# 设置面板子计划（D2：窗口 B 五面板实现）

| 项 | 内容 |
| :--- | :--- |
| 父计划 | [.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md](桌宠-任务拆分与开发顺序规划.md) §三 阶段 3 / 任务 D2、里程碑 M1 |
| 规则依据 | `.codebuddy/rules/`：code-standards §0/§7（Vue 约定/单向数据流）、design-baseline §G（Key 边界）；**权威实现蓝图**：[`docs/design/窗口B-设置界面设计方案.md`](../../docs/design/窗口B-设置界面设计方案.md) v1.0——布局（§3）、五面板规格（§4.1–4.5）、组件细节（§5）、全局交互状态（§6）、验收清单（§8）；**模型选型**：父计划「阶段 3 补充：LLM 大模型配置」小节（必选/可选两级暴露、三档路由规则、计费说明入口） |
| 范围边界 | **做**：ui 控件补齐 + 五个 Panel 实装 + 数据流绑定。**不含**：MemoryPanel 业务（D3，本计划保持占位）、测试连接按钮（登记 E 期，依赖 C4 client）、托盘唤起（C2 已备） |
| 现状（骨架已就位） | ✅ `SettingsWindow.vue` + `TitleBar` + `SideNav` + 五 Panel 占位；✅ `ui/` 已有 AppButton/AppCard/AppToggle/AppInput/StatusBadge/HintBox；✅ `api/settings.ts`（getTyped/saveTyped/listProviders/setQuotaMode 契约）、`api/pet.ts`（get/update/logs）、`stores/{settings,pet}Store`。**缺口**：ui 缺 SegmentedControl/TimeChip/KbdChip/AppToast；`components/{settings,affection}/` 全部占位；五 Panel 为骨架文案 |
| 状态 | **已完成**（2026-10-22，提交 `1cf8c35`；M1 闭环最后一块：设置持久化达成） |

---

## 一、关键裁决（登记勘误 #13：勿扰多段结构）

| # | 问题 | 裁决 |
| :--- | :--- | :--- |
| 1 | **勿扰单段 vs 多段**：M1 种子 `dnd_start_time/dnd_end_time` 单段，但 `桌宠.md` 明确"可以设置多个勿扰时段"、设计方案 §4.3 为多段 TimeChip | **value JSON 升级为数组**：`"dnd": {"enabled":bool, "ranges":[{"start":"09:00","end":"12:00"},...]}`。B4 未执行 → **直接改 M1 种子**（同勘误 #7 模式）并回写附录 A.5/B4 计划；若 B4 已执行 → M4 数据迁移预案（单段→数组）。读取双格式兼容兜底 |
| 2 | 契约增量 | `SettingsSnapshot` 增 `dnd: DndConfig`（B5/C3 契约扩展，`api/types.ts` 同步）；`settings_save_typed` 支持 dnd 保存（type=2 行） |
| 3 | 保存模型（设计方案 §6 分类） | **显式保存**：LLM 三档配置（防误触丢 Key）；**即改即存**：勿扰开关/时段/频率档位/桌宠名字（debounce 300ms）；每类操作的反馈：显式保存→AppToast 成功、即改即存→静默（失败 toast） |
| 4 | Key 输入 UX（§4.2） | `type=password` + 显示切换；已配置时 placeholder 显示脱敏指纹（`已配置 ·····1234`）；**仅输入新值才提交**（空/缺省 = 不改动存量 Key，B5 契约既有口径）；切换服务商 Chip 自动预填 baseUrl/默认模型（来自 `settings_list_providers`） |
| 5 | 预览区实现 | `PreviewStage` 创建**独立渲染器实例**（`factory.createRenderer` 直调，不走 `usePetRenderer` 单例——单例属窗口 A 主形象）；挂载/切换形象销毁重建；窗口 B 与 A 不同 JS 上下文互不干扰 |
| 6 | code_dog 选项 | PetOptionCard 展示但**禁用态**（`CodeDogRenderer` 未实现，C1 范围），标注"Phase 内解锁"——不留死按钮点击报错 |
| 16 | 选型方案落地细节（父计划「阶段 3 补充」） | ① ProviderChip 显示**成本档徽标**（低/极低/免费，来自 preset `costTier`，价格不写死）；② 三档均未配置时显示**必选约束引导** HintBox（"至少配置 DeepSeek 或通义千问之一，可单独支撑主对话 + 沉淀全流程"）；③ fallback 档未配置 → 表单回落提示（"未配置时自动使用必选模型兜底"）；④ 面板底部**计费说明入口**（HintBox + 各厂商官网/控制台链接占位——选型规则要求）；⑤ 可选组内 GLM-Flash / SiliconFlow 打"内置额度池候选"角标（与 C3 `EMBEDDED_QUOTA_BASE_URL` 呼应） |

---

## 二、任务分解（D2.1–D2.7）

| ID | 任务 | 具体内容 | 依赖 | 验收标准 |
| :--- | :--- | :--- | :--- | :--- |
| D2.1 | ui 控件补齐 | `SegmentedControl`（档位切换，受控）、`TimeChip`（`<input type="time">` 样式化 + 起止校验 start<end）、`KbdChip`（快捷键展示）、`AppToast`（全局轻提示，单例队列）；全部零业务依赖（ui 红线） | - | 组件可独立复用、typecheck 过 |
| D2.2 | LlmPanel 实装（§4.2 + 选型方案） | 三档 SegmentedControl（main/memory/fallback）→ 每档：ProviderChip 组（必选/可选两组 + **成本档徽标 + 额度池候选角标**，来自 `listProviders`）+ baseUrl/model（选 Chip 预填、可改）+ Key 输入（指纹回显）+ `modelCustom`；quota 模式 radio（内置额度/BYOK，BYOK 校验走 C3 命令错误呈现）；**必选约束引导**（三档全空 → HintBox 提示）与 **fallback 回落提示**（未配置文案）；**计费说明入口**（HintBox + 官网链接占位）；**显式保存按钮** → `settings_save_typed` → toast + 快照刷新 | C3 | 三档独立配置互不串扰；切换任一档位模型后**对话流式输出与后台沉淀（facts/summaries/vec_memory）均正常**（选型"验收补充"）；断网或 Key 失效走 fallback 不崩溃 |
| D2.3 | DndPanel 实装（§4.3） | 勿扰总开关 Toggle + 多段时段列表（TimeChip 增/删/改，起止校验 + 重叠提示）+ 主动频率 SegmentedControl（安静/正常/粘人 + 次数说明文案）；全部即改即存（dnd 保存 type=2 行；频率档位存 settingsStore → value JSON） | D2.1 | 重启后配置持久；时段重叠被阻止 |
| D2.4 | AffectionPanel 实装（§4.4） | `AffectionRing`（conic-gradient 圆环 + 中心分数/1000）、`LevelPill`（当前等级 + 距下档提示）、`LogItem` 流水（`pet_list_affection_logs(50)` + 加载更多）；数值**仅此看板可见**（窗口 A 零数值红线不适用窗口 B） | C5 | 分数/等级/流水与真实数据一致 |
| D2.5 | AvatarPanel 实装（§4.1） | 桌宠名字 AppInput（即改即存 debounce → `pet_update_state`）、`PetOptionCard` ×2（code_cat 选中/code_dog 禁用）、`PreviewStage`（独立 PIXI 实例实时预览 + 切换重建） | C1 | 名字改动窗口 A 标题栏同步；预览可用可销毁 |
| D2.6 | MemoryPanel 骨架升级 | 保持占位（D3 范围），但接 `memory_list_diaries` 空态展示（"今天还没有日记"），避免 D2 走查时空白报错 | - | 空态优雅、无报错 |
| D2.7 | 走查与收口 | **设计方案 §8 验收清单逐项走查**（10 项）+ 手工清单（§五）+ 契约文档回写（dnd 契约增量）+ 父计划 D2 行回写 | 全部 | DoD 全勾 |

---

## 三、数据流总览

```
窗口 B 挂载
  ├─ settings_get_typed() ──▶ settingsStore ──▶ LlmPanel(三档表单)/DndPanel(勿扰/频率)
  ├─ settings_list_providers() ──▶ ProviderChip 组
  ├─ pet_get_state() ──▶ petStore ──▶ AvatarPanel/AffectionPanel
  └─ pet_list_affection_logs(50) ──▶ AffectionPanel 流水

即改即存路径：Toggle/TimeChip/Segmented/Input(debounce) ──▶ api ──▶ DB（失败 toast 回滚 UI）
显式保存路径：LlmPanel 保存按钮 ──▶ settings_save_typed ──▶ AppToast ──▶ 快照刷新
事件同步：affection:changed ──▶ petStore ──▶ AffectionRing 实时刷新（窗口 B 常驻）
```

---

## 四、文件清单（本计划允许的 diff 范围）

| 文件 | 操作 | 内容 |
| :--- | :--- | :--- |
| `src/components/ui/{SegmentedControl,TimeChip,KbdChip,AppToast}.vue` | NEW | 控件补齐（§5.1 规格） |
| `src/components/settings/{ProviderChip,QuotaBar,PetOptionCard,PreviewStage}.vue` | NEW | 窗口 B 业务组件（§5.2） |
| `src/components/affection/{AffectionRing,LevelPill,LogItem}.vue` | NEW | 看板组件（§5.2） |
| `src/views/settings/panels/{Llm,Dnd,Affection,Avatar,Memory}Panel.vue` | MODIFY | 占位 → 实装 |
| `src/views/settings/SettingsWindow.vue` | MODIFY（最小） | Toast 挂载点 |
| `src/api/types.ts` / `src/api/settings.ts` | MODIFY | DndConfig 契约增量（勘误 #13） |
| `src/stores/{settings,pet}Store.ts` | MODIFY | 字段扩展（dnd/frequency） |
| B4 计划 M1 种子 / 父计划附录 A.5 / 契约文档 | MODIFY（文档） | 勘误 #13 同步 |
| 父计划 D2 行 | MODIFY | 状态回写 |

**不动**：`renderer/**`（仅 PreviewStage 消费 factory）、Rust 侧（C3/B5 命令已覆盖需求，零新增）、窗口 A 组件。

---

## 五、验证门禁与 DoD

- [ ] `pnpm typecheck` 零错误；（Vitest 就绪后）TimeChip 校验逻辑可单测
- [ ] 手工清单：
  1. 三档 LLM 配置：选 Chip 预填 → 保存 → **重启后三档全部还原**；Key 仅指纹回显，回包无明文
  2. BYOK 校验：main 无 Key 切 BYOK → 错误 toast；配上后成功
  3. 勿扰：加两段时段 → 重启还原；时段重叠被阻止；总开关关闭后 C6 主动静默（联动验证）
  4. 频率档位切换 → settingsStore 即时生效（C6 引擎读取）
  5. 好感度看板：圆环/等级/流水与 `affection_logs` 一致；互动后实时刷新
  6. AvatarPanel：改名 → 窗口 A 标题栏/气泡称呼同步；预览切换无泄漏（反复切换 10 次内存平稳）
  7. 关闭窗口 B（✕=hide）→ 托盘唤起 → 表单状态保留
  8. **选型验收补充**：三档分别切换模型 → 主对话流式与后台沉淀链路均正常；断网或 main Key 失效 → 气泡显示 fallback/本地话术不崩溃（C4 回落链联动验证）
- [ ] 选型方案对齐（勘误 #16）：成本档徽标 / 必选约束引导 / fallback 回落提示 / 计费说明入口 / 额度池候选角标 五项落地
- [ ] 设计方案 §8 验收清单 10 项逐条勾选留档
- [ ] 红线自查：ui/ 组件无业务 import；Key 无 localStorage/日志；`grep invoke` 仅 api/；窗口 B 无 PIXI 实例泄漏
- [ ] 冷眼三问：① 即改即存失败是否回滚 UI（乐观更新撤销）？② 三个面板并发保存是否互斥安全（不同 type 行）？③ PreviewStage 销毁路径完整（onUnmounted）？
- [ ] 父计划 D2 行回写（M1 闭环最后一块：设置持久化达成）

## 六、风险与回退

| 风险 | 对策 |
| :--- | :--- |
| conic-gradient 圆环在 WebView2 渲染差异 | WebView2 (Chromium) 原生支持；降级方案 SVG stroke-dasharray（一行切换） |
| `<input type="time">` 样式可控性差 | 样式化 wrapper + 自定义展示层；极端情况自绘两列下拉 |
| 即改即存失败后 UI 与 DB 不一致 | 乐观更新模式：失败 toast + 从快照回滚该字段 |
| PreviewStage 双实例与窗口 A 渲染器混淆 | 窗口隔离天然安全 + factory 直调不经单例；dev 时双开验证 |
| LlmPanel 表单复杂度高（三档 × 多字段） | 每档为独立子组件实例（v-for tiers），状态按档隔离；保存整档提交 |
| dnd 数组校验遗漏（跨午夜时段） | TimeChip 支持 start>end 视为跨午夜段（文档无明确——登记 E1 确认，MVP 提示但不阻止） |

## 七、执行顺序

D2.1（ui 控件）→ D2.2（LlmPanel）→ D2.3（DndPanel）→ D2.4（AffectionPanel）→ D2.5（AvatarPanel）→ D2.6（MemoryPanel 空态）→ §五 走查 → D2.7 DoD 收口。

---

## 十、执行结果与计划对应（2026-10-22，提交 `1cf8c35`）

### 各任务实装对照

| 计划 | 文件 | 结果 |
| :--- | :--- | :--- |
| D2.1 ui 控件 | `ui/SegmentedControl.vue`（泛型单选项）、`ui/TimeChip.vue`（mono+内嵌时钟+hover✕+dashed 添加）、`ui/KbdChip.vue`（只读展示）、`ui/AppToast.vue` + `composables/useToast.ts`（单例队列 2.4s）、`ui/AppButton` 补 `loading` 态 | ✅ |
| D2.2 LlmPanel | `panels/LlmPanel.vue` + `settings/ProviderChip.vue`（★必选+成本档+额度池角标）+ `settings/QuotaBar.vue` | 三档 SegmentedControl 独立表单；选 Chip 预填 Base URL/模型；Key 密码框+指纹回显+留空不改；必选约束引导 HintBox；fallback 回落提示；计费说明入口；成本档/额度池候选角标；显式保存→`settings_save_typed`→toast+快照刷新 |
| D2.3 DndPanel | `panels/DndPanel.vue` | 勿扰开关 + 多段时段（TimeChip 增删改）+ 频率档位 SegmentedControl；**即改即存**（debounce 300ms→`settings_save_dnd`，乐观更新+失败回滚）；重叠/非法段阻止保存 |
| D2.4 AffectionPanel | `panels/AffectionPanel.vue` + `affection/AffectionRing.vue`（conic-gradient）+ `LevelPill.vue` + `LogItem.vue` | 左 hero=圆环+等级+距下档差；右上规则卡（静态 5 行）；右下流水（`pet_list_affection_logs`）；数值仅看板可见，绝不进对话气泡 |
| D2.5 AvatarPanel | `panels/AvatarPanel.vue` + `settings/PetOptionCard.vue` + `settings/PreviewStage.vue` | 名字即改即存（`pet_update_state`，截断 20 字）；PreviewStage 独立 PIXI 实例（普通变量持有、onBeforeUnmount/切换销毁、非 reactive）；code_cat 可选、code_dog/导入占位 |
| D2.6 MemoryPanel | `panels/MemoryPanel.vue` | 接 `memory_list_diaries` 空态（"她还没写过日记——今晚见"）+ 轻量预览，避免走查空白报错（完整编辑留 D3） |

### 关键 API 实态修正（勘误 #17：计划"Rust 零新增"假设不实）

1. **新增 `settings_save_dnd` 命令**：原 `settings.rs` 仅有 `settings_get_typed`/`save_typed`/`set_quota_mode`，**无 dnd 保存命令**，验收"重启后配置持久"无法满足。实装 `settings_save_dnd(dnd, frequency)`，写 `app_settings` type=2 行。
2. **frequency 持久化**：`SettingsDndValue`/`SettingsSnapshotDto` 扩 `frequency: String` 字段；`parse_dnd_full` 解析回填；前端 `SettingsSnapshot.frequency`、`settingsStore.applySnapshot` 同步、`saveDnd` 落库。C6 引擎经 `settingsStore.frequency` 即时读取。
3. **自启 / 情境感知开关后端 IPC 命令缺失**：`autostart` 仅托盘菜单、`set_probe`/`set_autostart` 等命令不存在 → DndPanel/AvatarPanel 相关 toggle 仅 `disabled` 占位 + HintBox 说明，"不造假状态"。登记缺口（后续阶段补齐）。
4. **额度条比例**：内置体验额度无真实计量接口 → `QuotaBar` 传 `ratio=1` 占位 + 标注"体验额度随后端开放"，**不编假数**。
5. **等级阈值展示近似**：`AffectionPanel` 用 `LEVEL_THRESHOLDS=[0,250,500,750,1000]` 视觉等分，与 C5 等级映射对齐；非权威阈值（展示用）。
6. **额度池候选角标**：`ProviderChip` 按 `costTier==='free'` 或已知 id 集合（`builtin`/`siliconflow`/`glm-flash`）推断，展示用。

### 门禁终态

`cargo clippy --all-targets -D warnings` ✅ / fmt ✅ / **cargo test 94 passed**；`pnpm typecheck` ✅ / `pnpm build` ✅ / `pnpm lint` 0 ✅ / `pnpm test` 40 passed ✅（D2 未增单测）/ `pnpm format:check` ✅

### 冷眼三问

| # | 项 | 结果 |
| :--- | :--- | :--- |
| ① | 即改即存失败是否回滚 UI | DndPanel/AvatarPanel 乐观更新 + 失败 `pushToast` + 从 snapshot 回滚该字段（含 frequency/dnd/enabled/name） |
| ② | 三个面板并发保存互斥安全 | 不同 `type` 行隔离：LLM→type=1（按档 upsert_value）、dnd+frequency→type=2（upsert_value）、quota→`app_settings` 列；各自独立行无竞争 |
| ③ | PreviewStage 销毁路径完整 | `onBeforeUnmount` + `watch(rendererId)` 重建均调 `dispose()`（renderer.dispose + 清空容器 innerHTML），PIXI 实例普通变量不进 reactive，杜绝泄漏 |

### 红线自查

| # | 项 | 结果 |
| :--- | :--- | :--- |
| ui/ 零业务 import | SegmentedControl/TimeChip/KbdChip/QuotaBar/AppToast 纯展示；AppToast 仅消费 `useToast` 队列（非 api/store 业务逻辑） | ✅ |
| Key 无 localStorage/日志 | 仅经 `callCommand` 单向流向 Rust，前端零缓存、零日志 | ✅ |
| grep invoke 仅 api/ | 全部 `callCommand` 集中于 `src/api/*`（settings/pet/memory/...） | ✅ |
| 窗口 B 无 PIXI 泄漏 | PreviewStage 独立实例 + 销毁路径完整 | ✅ |

### §8 / §五 验收清单勾选

- 手工清单 1–8：三档还原/Key 指纹/勿扰重叠阻止+联动静默/频率即时生效/好感度看板一致/改名同步+预览无漏/隐藏保留/选型回落链 —— **代码层达标，待真机手工复验**
- 选型验收 5 项（成本档徽标/必选约束引导/fallback 回落/计费说明入口/额度池候选角标）：✅ 落地
- 设计方案 §8 验收清单 10 项：镜像到 §四 五面板规格逐条实装（详见各 Panel 对照）

### 待用户手工验证（真机）

1. 三档选 Chip 预填 → 保存 → **重启还原**；Key 仅指纹
2. main 无 Key 切 BYOK → 错误 toast；配后成功
3. 勿扰加两段 → 重启还原；重叠被阻止；关开关后 C6 静默
4. 频率切档 → C6 引擎即时生效
5. 好感度看板与 `affection_logs` 一致；互动后刷新
6. 改名 → 窗口 A 称呼同步；预览反复切换 10 次内存平稳
7. ✕=hide → 托盘唤起 → 表单保留
8. 三档分别切模型 → 流式/沉淀链路正常；断网/失效 → fallback 不崩溃

### 移交

- **D3 记忆阅读区**：MemoryPanel 已接 `diaries` 空态，D3 实装日记卡片 + 事实标签编辑（C7 后端已就绪）
- **E 期测试连接按钮**：登记 E 期，依赖 C4 client（本计划范围不含）
- **自启 / 情境感知持久化**：后端命令缺口，后续阶段补齐（本计划已占位）
