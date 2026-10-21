# 窗口 B · 设置界面设计方案 v1.0

| 项 | 内容 |
| :--- | :--- |
| 文档目的 | 为「七七」窗口 B（Settings 设置层）提供可落地的界面设计方案：布局说明、样式规范、组件细节、交互状态与设计决策依据 |
| 视觉来源 | `docs/design/pet-design/index.html`（Jelly-Toy 果冻奶油风设计稿，以下称"设计稿"） |
| 需求来源 | `docs/design/桌宠.md`（§3.1 双窗口、模块 3/8/9）、`.codebuddy/plans/桌宠-任务拆分与开发顺序规划.md`（D2/D3、C3、「LLM 大模型配置」小节） |
| 技术栈 | Tauri 2 + Vue 3 `<script setup lang="ts">` + TypeScript strict（**设计稿中的 React 字样一律按 Vue 实现**，见 `CODEBUDDY.md`） |
| 关联任务 | 计划文档 D2（设置面板）、D3（记忆阅读区）、C3（零门槛配置后端） |
| 版本 / 日期 | v1.0 · 2026-10-21 |

---

## 一、设计定位与目标

窗口 B 是**唯一的常规配置与信息窗口**：承载形象管理、大模型配置、勿扰与主动频率、好感度看板、记忆阅读区五大板块。关闭仅 `hide` 不销毁，数据全部经 IPC 读写 SQLCipher。

设计目标（优先级从高到低）：

1. **零门槛**：服务商预设自动填充、开箱即用的内置额度，让 90% 用户不改配置也能跑通对话。
2. **隐私可见**：把"Key 加密存储、不截屏、记忆可纠正"做进界面里，让用户**看得见**安全边界（侧边栏脚注 + 加密徽标 + 置信度标签）。
3. **玩具感**：Jelly-Toy 果冻奶油风——暖奶油纸底、抹茶绿主色、大圆角、软阴影、微弹动效，与桌宠"有生命"的定位一致。
4. **低打扰**：信息按卡片分块、每屏一个主题；好感度只呈现"情绪化叙事"，绝不播报冰冷数值（规则见 `design-baseline.mdc` §F）。

非目标（明确不做）：多主题/暗色模式（MVP 不做）、窗口 B 内嵌完整聊天记录浏览器、多角色管理（单用户单角色，`pet_states.id` 固定 1）。

---

## 二、视觉风格（样式规范）

### 2.1 Design Tokens（与设计稿 `:root` 一一对应，实现时提取到 `src/styles/tokens.css`）

**色彩**

| Token | 值 | 用途 |
| :--- | :--- | :--- |
| `--bg-page` | `#EFE7D8` | 页面底色：暖奶油纸感（窗口 B 内不直接使用，用于外层） |
| `--bg-window` | `#FFFEF9` | 窗口底色 / 卡片白 |
| `--bg-soft` | `#FBF5EA` | 侧边导航、hint 浅底 |
| `--bg-deep` | `#F4EDDE` | 内嵌区块底（segmented 容器、quota 轨道、foot） |
| `--ink` | `#403A2E` | 主文字：暖炭棕（正文、标题） |
| `--ink-soft` | `#948A75` | 次级文字（说明、label、时间戳） |
| `--line` | `#EBE1CE` | 分割线、输入框描边、卡片边框 |
| `--matcha` | `#7CA65B` | 主色：抹茶绿（主按钮、选中态、nav 激活、toggle on） |
| `--matcha-deep` | `#5D8541` | 主色深（hover、强调文字、三角指示器） |
| `--matcha-soft` | `#EAF1DF` | 主色浅底（badge green、selected 卡片底） |
| `--peach` | `#FF9E5E` | 点缀：蜜桃橙（好感度数字、NEW 标签、装饰） |
| `--peach-soft` | `#FFEBDB` | badge orange 底 |
| `--butter` | `#FFD98A` | 点缀：奶油黄（铃铛、渐变条） |
| `--blush` | `#F5AEB9` | 点缀：腮红粉（删除按钮 hover） |
| 语义色 | `#E05B4C`（危险红）/ `#D4647A`（badge pink）/ `#F26D5B`（关闭按钮 hover） | 扣分流水、破坏性操作 |

**字体**

| Token | 值 | 用途 |
| :--- | :--- | :--- |
| `--font-disp` | `'ZCOOL QingKe HuangYou','Noto Sans SC',sans-serif` | 展示字体：标题、面板名、数字（圆润玩具感） |
| `--font-body` | `'Noto Sans SC',-apple-system,'Microsoft YaHei',sans-serif` | 正文 |
| `mono` | `Consolas,monospace` | Base URL / Key / 时间 / 分数值（`.input.mono`、`.kbd`、`.delta`） |

> ⚠️ 落地决策：Google Fonts 在 Tauri 离线场景不可靠。两款字体需**下载 woff2 随应用打包**（`src/assets/fonts/`，`@font-face` 本地声明），失败回退 `Microsoft YaHei`。

**几何与质感**

| Token | 值 | 说明 |
| :--- | :--- | :--- |
| 圆角 | `--r-sm:10px` / `--r-md:16px` / `--r-lg:22px` | 输入框 12px、按钮 12px、卡片 22px、导航项 13px、chip 11–12px、胶囊 999px |
| `--shadow-card` | `0 2px 6px rgba(93,84,60,.06), 0 10px 26px rgba(93,84,60,.07)` | 卡片软阴影 |
| `--shadow-window` | `0 4px 12px rgba(64,58,46,.10), 0 34px 80px rgba(64,58,46,.22)` | 窗口投影（Tauri 原生阴影接管后仅作参考） |
| 纸张噪点 | `radial-gradient(rgba(120,105,70,.10) 1px, transparent 1px)` 5×5 平铺，`opacity:.5` + `multiply` | 铺在窗口内容区最底层，避免大面积纯色呆板 |
| 描边 | 卡片 `1.5px solid var(--line)`；输入框 `1.5px`，focus 变 `--matcha` + `0 0 0 3.5px rgba(124,166,91,.18)` 外圈 | 全局统一 1.5px 线宽 |
| 立体细节 | 主按钮/激活导航 `inset 0 -2px 0 rgba(0,0,0,.12)`；`.kbd` `border-bottom-width:3px` | "软糖"压边，玩具感来源之一 |

**动效规范**

| 名称 | 参数 | 用途 |
| :--- | :--- | :--- |
| `panelIn` | `320ms cubic-bezier(.2,.9,.3,1.2)`，`translateY(10px)→0` + 淡入 | Tab 面板切换 |
| `bubbleIn` | `600ms cubic-bezier(.2,1.2,.4,1)`（延迟 300ms） | 气泡入场（窗口 A） |
| toggle 滑块 | `220ms cubic-bezier(.3,1.4,.5,1)`（过冲弹性） | 开关 |
| 按钮 active | `scale(.96)`，`120ms` | 全部按钮 |
| hover 位移 | 卡片 `translateY(-3px)` + 阴影加深（160ms） | 可选卡片 |
| `breath` / `jelly` | 见 §4.1（**仅预览舞台 CSS 近似；窗口 A 真实果冻物理由 PIXI Ticker 实现**） | 预览示意 |

### 2.2 视觉语言关键词

- **卡片即模块**：每个功能块一张 `.card`（白底 + 22px 圆角 + 1.5px 边框 + 软阴影），卡片标题前固定一枚**旋转 45° 的抹茶绿小菱形**（8×8，`border-radius:3px`）——全站唯一的"模块标记"，不可替换为其他图标。
- **胶囊化标签**：状态一律用 `.badge`（green/orange/pink 三色），不打断阅读流。
- **暖色单主色**：抹茶绿是唯一"操作色"（激活、保存、开启）；蜜桃橙只做点缀与好感度数字，不用于操作；红色仅用于扣分与删除确认。

### 2.3 可访问性基线

- 交互控件必须有 `aria-label`（设计稿已示范）与 `:focus-visible` 可见焦点（复用输入框 focus 外圈样式）。
- `--ink-soft` 在白底上对比度约 3.5:1，**仅用于 ≥11px 的辅助说明**；关键操作反馈、数值、按钮文字用 `--ink` 或白字深底。实现时用对比度工具复核，不达 AA 的组合只允许出现在装饰性文本。
- 全键盘可达：Tab 顺序 = 标题栏 → 侧边导航 → 当前面板；导航支持 ↑/↓ 切换（详见 §5.0）。

---

## 三、窗口规格与整体布局

### 3.1 窗口规格

| 属性 | 值 | 依据 / 决策 |
| :--- | :--- | :--- |
| 默认尺寸 | **1000 × 660 逻辑像素** | 设计稿 meta-chip 标注「窗口 B · 1000×660」为权威；CSS 中 `.window{width:950px}` 仅为舞台内示意宽度 |
| 最小尺寸 | 860 × 600 | 侧边导航 212 + 内容区最小 648 |
| 缩放 | 允许 resize，尺寸记忆写入 `app_settings`（不引入 window-state 插件） | 与"集中配置"原则一致 |
| 关闭行为 | 点关闭按钮 = `hide()`，托盘可再次唤起；进程退出仅走托盘「退出」 | `桌宠.md` §3.1「关闭时仅 hide，不销毁」 |
| 标题栏 | **自绘**（`decorations:false` + `data-tauri-drag-region`），高 46px | 见 §5.0 决策 |
| 主题 | 固定浅色 | MVP 非目标 |

### 3.2 布局结构（ASCII 示意）

```
┌──────────────────────────────────────────────────────────────────────┐
│ titlebar 46px  [logo 喵喵 · 设置  PET SETTINGS]      [─ ][□ ][✕=hide] │
├──────────────┬───────────────────────────────────────────────────────┤
│ sidenav 212px│ content（overflow-y:auto，padding 26 30 30）           │
│ bg-soft      │                                                        │
│ ┌──────────┐ │  panel-head  h2(23px 展示体) + p(12px 说明)             │
│ │设置分类   │ │  ┌──────────────────────────────┐                     │
│ │● 形象管理 │ │  │ card …（每屏 1–2 张卡片）      │                     │
│ │◇ 大模型   │ │  └──────────────────────────────┘                     │
│ │🌙 勿扰主动 │ │  ┌──────────────────────────────┐                     │
│ │♥ 好感度   │ │  │ card …                        │                     │
│ │📖 记忆 NEW│ │  └──────────────────────────────┘                     │
│ └──────────┘ │                                                        │
│ foot 隐私红线 │                                                        │
└──────────────┴───────────────────────────────────────────────────────┘
```

- **侧边导航**（212px，`--bg-soft`，右侧 1px 分割线）：`nav-label`（10px/900/字距 2.5）→ 5 个 `.nav-item` → 底部 `.foot` 隐私声明卡（`margin-top:auto`，虚线边框）。激活项：抹茶绿实底白字 + 左侧 `--matcha-deep` 小三角指示 + 投影；`NEW` 标签用 `.tag` 胶囊。
- **内容区**：右上角一抹奶油黄径向渐变（`radial-gradient(400px 200px at 100% 0%, rgba(255,217,138,.16))`）；面板切换带 `panelIn` 动画；滚动条建议 8px 圆角滑块（`--line` 底、`--ink-soft` 滑块）。
- **窄窗口适配**（<1080 逻辑像素宽，对应最小 860）：侧边导航转**横向图标条**（隐藏 label 与 foot），`.avatar-layout / .aff-layout / .mem-layout / .freq-grid / .row2` 全部降为单列——沿用设计稿 `@media` 规则。

---

## 四、五个 Tab 的布局说明与数据绑定

> 数据绑定列出的表/字段为唯一真源（`CODEBUDDY.md`「Configuration Management」）；前端所有读写经 `src/api/` 的 invoke 封装，API Key 全程不明文回显（§4.2）。

### 4.1 Tab 1 · 形象管理（默认面板）

**布局**：`grid-template-columns: 296px 1fr; gap:14px`

| 区域 | 内容 | 数据 / 交互 |
| :--- | :--- | :--- |
| 左：预览卡 `preview-card` | 渐变暖底（`#FFFDF4→#FDF3DD`）；`preview-stage` 210px 高：左上 `LIVE PREVIEW · 窗口 A` 角标、中央角色、底部提示"点击角色体验果冻物理"；下方名字（22px 展示体）+ 副标 + 三枚 badge（果冻物理/视线跟随/点击穿透） | **预览区挂载真实 PIXI 实例**（复用 `CodeCatRenderer`，`IPetRenderer.mount()`），点击调 `playAction('click')`。注意：窗口 A/B 是两个 WebView，各持独立 `Application`，每窗口一个 Ticker，符合规范。**禁止**使用设计稿的 `assets/pet-*.jpg`（非运行时资源）；PIXI 初始化失败则显示纯 CSS 占位猫，不阻塞面板 |
| 右上：选择内置形象 `pet-grid` | 3 列网格：`猫猫 code_cat · 奶油黄`（selected）、`狗子 code_dog · 抹茶绿`、`＋导入自定义形象`（dashed 虚线卡，标注 Phase 3 · CustomSpriteRenderer，点击仅 toast「Phase 3 敬请期待」） | 选中写入 `pet_states.active_renderer` / `renderer_config`；选中态：抹茶绿边框 + 浅绿底 + 右上「✓ 使用中」角标 |
| 右下：窗口与交互卡 | `row2`：桌宠名字 input（maxlength 20 → `pet_states.pet_name`，标题栏实时联动）+ 窗口尺寸 readonly（`400 × 400`）；两个 `set-row` 开关：边缘吸附休眠（§3.6）、头顶铃铛快捷勿扰 | toggle 状态即设置值，变更即存（`app_settings` type=2），无需单独保存按钮 |

**状态**：名字输入超 20 字符截断并抖动提示；切换形象时预览区 240ms 淡入过渡。

### 4.2 Tab 2 · 大模型配置（本方案重点，与计划文档「LLM 大模型配置」小节对齐）

**布局**：单列两张卡片。

**卡片 1 · 服务商与凭据**

```
┌ 服务商预设 ────────────────────────────────────────────────┐
│ 用途档位 segmented：[ 主对话 main ] [ 记忆沉淀 memory ] [ 兜底 fallback ] │
│ provider-chips（按档位分组渲染）：                          │
│  ★必选：DeepSeek · 通义千问    可选低成本：GLM-Flash · SiliconFlow │
│         · 豆包 Lite · 混元 Lite · 文心 Speed/Lite   其他：内置额度 · 自定义 │
│ row2: Base URL（mono，选中预设自动填充，可改） | 模型名称（mono） │
│ API Key（type=password + 👁 明文切换，placeholder sk-…）      │
│ [保存并测试连接 primary] [清空 Key ghost]   🔒 已由系统凭据管理器加密 │
└─────────────────────────────────────────────────────────────┘
```

- **三档用途**（`main`/`memory`/`fallback`）用设计稿的 `.segmented` 分段器呈现，切换后整卡内容切到该档位的配置；三档各自独立保存（`app_settings.value` JSON 内按档位分组存储）。
- **服务商 chips** 沿用 `.p-chip`（选中：`--ink` 深底白字 + 投影；`.dot` 圆点用 `--peach`），必选组前置一颗 ★ 且 chips 始终展示；可选组默认收进「＋ 添加低成本服务商」展开区，减少首屏噪音。选中 chip 自动回填 Base URL 与推荐模型（沿用设计稿 `initProviderChips` 行为）；推荐模型按计划文档：DeepSeek `deepseek-chat`、通义 `qwen-plus`（memory 档默认 `qwen-flash`）。
- **API Key**：`type=password` + 显隐切换；**保存后输入框清空并显示脱敏指纹**（如 `sk-****…****3f2a`），回显时只显示指纹不回显明文（前端零明文红线）。`🔒 已由系统凭据管理器加密` 为常驻 green badge。
- **测试连接**：点击后按钮进入 loading（文案→「连接中…」，禁用），Rust 侧发一次轻量请求，成功→green badge「连接正常 · 模型 xxx」，失败→红字错误行（含 HTTP 状态与建议：检查 Key/网络/余额）。

**卡片 2 · 内置体验额度（`is_custom_quota_used`）**

- 顶部文案行 + 剩余百分比（`--peach` 加粗）+ `quota-bar`（10px 圆角轨道，填充为 `butter→peach` 渐变，宽度=剩余比例）。
- `set-row` 开关「使用内置额度」：关闭即完全切 BYOK。**额度耗尽状态**：进度条 0、开关自动关闭、badge 变 pink「额度已用完」并引导切自有 Key；`fallback` 档位生效提示。

**状态矩阵**：未配置 Key（空状态）→ 卡片顶部显示 hint「先选服务商，粘贴 Key 即可开始」；三档均未配置任何 Key → 窗口 A 对话入口置灰并由气泡引导到本页。

### 4.3 Tab 3 · 勿扰与主动

**布局**：`freq-grid`（`grid-template-columns:1fr 1fr; gap:14px`），左右两列各两张卡片。

| 卡片 | 内容 | 数据 / 交互 |
| :--- | :--- | :--- |
| 勿扰时段 | 「启用勿扰」toggle（→ `dnd_enabled`）+ 多段 `time-chip` 列表（等宽 mono 字体 + 时钟 icon）+ 虚线「＋ 添加时段」 | 时段写入 `dnd_start_time`/`dnd_end_time`（value JSON 支持多段）；添加弹出 2 个 `input type=time` 的行内编辑器；chip 悬停显示 ✕ 删除（二次确认 toast） |
| 全局快捷键 | 3 个 `set-row`：唤起输入框 `Ctrl+Shift+P`、隐藏/显示桌宠 `Ctrl+Shift+H`、开机自启 toggle | `.kbd` 胶囊只读展示（MVP 不支持改键，subtitle 注明）；自启 → autostart 插件 |
| 主动互动频率 | `.segmented` 三档：安静（偶尔想你）/ 正常（刚刚好）/ 粘人（挂你身上） | 档位决定微随机区间：安静 1~2 次/天、正常 4~6 次/天、粘人 10+ 次/天；下方 `.hint`（🐙 图标）解释"时间窗口内随机触发，拒绝准点打卡" |
| 情境感知 | 3 个 `set-row` toggle：工作/游戏让路（主动概率 −80%）、空闲 30 分钟睡觉/回归伸懒腰、连续 3 天未互动触发傲娇 | 对应 Rust 探针开关（probe 配置持久化于 `app_settings`） |

### 4.4 Tab 4 · 好感度看板

**布局**：`aff-layout`（`grid-template-columns:300px 1fr`）。

| 区域 | 内容 | 数据 / 交互 |
| :--- | :--- | :--- |
| 左：等级 hero `aff-hero` | 渐变暖底卡：`aff-ring` 好感度圆环（**`conic-gradient` 实现**，进度色 `--peach`、轨道 `#EDE2CB`；中心白圆内 40px 展示体数字 + `of 1000`）；下方 `lv-pill` 深棕胶囊「Lv.3 亲密」；`aff-next` 行「距离 Lv.4 羁绊 还差 159 分 · 会撒娇、会表达想念」；两枚 badge：弹性衰减已启用 / 禁止播报数值 | 读 `pet_states.affection_points` + `relationship_level`。**等级→表现文案**映射内置（陌生/熟人/亲密/羁绊，见 baseline §F）。圆环数字与差值是**唯一的数值暴露点**（仅看板内可见，绝不出现在对话/气泡中）。数字变化用 800ms 缓动滚动动画增强"成长感" |
| 右上：加减分规则卡 | `rule-list` 5 行 `set-row`：每日打卡 +10 / 抚摸(日上限 20) +1 / 深度对话·共鸣 +5~+20 / 超 3 天未互动逐日 -5（止于等级下限）/ 触犯禁忌 -10~-30 | 静态规则展示，`.delta.plus` 绿 / `.delta.minus` 红（mono 字体） |
| 右下：近期变动流水卡 | `log-item` 列表：32px 圆角图标位（g/o/p 三色底 + emoji）+ 标题 + 时间·原因 + 右侧 delta | 读 `affection_logs`（分页，每页 20，滚动加载）。**衰减条目文案必须情绪化**：如「两天未互动 · 保护性衰减（停在 Lv.3 下限）」副标「绝不向你播报，她只是有点委屈」——这是本 Tab 的红线样例 |

**状态**：流水为空 → hint「还没有流水，去摸摸她吧」；加载中骨架屏（3 行 `log-item` 灰条）。

### 4.5 Tab 5 · 记忆阅读区（D3）

**布局**：`mem-layout`（`1fr 1fr`）。

| 区域 | 内容 | 数据 / 交互 |
| :--- | :--- | :--- |
| 左：日记列 | `diary-card`：头部 📖 + 「猫猫日记」+ 日期·天气；正文 12.5px/行高 2（第一人称）；右上角超大引号水印（`110px, rgba(255,158,94,.16)`）；foot：`badge green`「心情 88 · 期待」+「重新生成」ghost 小按钮 | 读 `diaries`（`diary_date` 倒序，日期切换器）；「重新生成」二次确认后调后台 LLM 重写当日日记；心情由 `mood` 字段映射文案。下方 `.hint`（🌙）说明生成时机（每日结束/长时间离开，读当日 `messages`） |
| 右：事实标签卡 `mem-chips` | 三个 `mem-group`：喜好 PREFERENCE / 日程 EVENT / 档案 PROFILE（组标题前蜜桃橙圆点）；每组 `chip-row` 里若干 `fact-chip`：内容 + `conf` 置信度胶囊（如 98%）+ ✎ 编辑 + ✕ 删除（del hover 变红） | 读 `facts`（按 `category` 分组，`confidence` 降序）。**编辑**：行内 input 替换文本，回车/失焦保存；**删除**：hover 变腮红粉，点击转确认态（✕→「确认?」）防误删。两者均走 `memory_*` 命令，后端**同步重建 `vec_memory` 向量**（计划 C7）。下方 `.hint`（✏️）说明纠正权与置信度含义 |
| 列表脚注 | 日记列顶部增加日期导航（‹ 昨天 ›）与「暂无日记」空态 | 空态文案：「她还没写过日记——今晚见。」 |

---

## 五、组件细节（组件规格 + Vue 组件映射）

### 5.0 全局壳与窗口铬件

| 组件 | 规格 | 决策 |
| :--- | :--- | :--- |
| `SettingsWindow.vue`（视图壳） | `flex column`：titlebar + body（sidenav + content） | 路由用组件切换（非 vue-router），当前 tab 存于内存，恢复显示时回到上次 tab |
| `TitleBar.vue` | 高 46px，渐变底 `#FFFDF6→#FBF4E6`，底部 1px `--line`；左：26px 抹茶圆角 logo（猫脸 SVG，内嵌 `#5D8541` 眼睛）+「{pet_name} · 设置」17px 展示体 + `PET SETTINGS` 11px 字距 2 副标；右：三枚 46px 方形按钮 | **自绘标题栏**（`decorations:false` + `data-tauri-drag-region`）：① 关闭按钮需执行 `hide` 而非系统 close；② 保持果冻风统一。最小化/最大化/关闭 hover：`rgba(124,166,91,.14)`；**关闭 hover：`#F26D5B` 白叉**。需在 `capabilities` 增补 `core:window` 相关权限。⚠️ 若自绘导致 Win10 圆角/阴影缺失，回退方案：保留系统 decorations，仅把关闭拦截为 hide |
| `SideNav.vue` / `NavItem.vue` | 见 §3.2；图标统一 17px、`stroke-width:2`、圆头线条 SVG（与设计稿同一套，禁用第三方图标库） | 键盘：↑/↓ 在导航项间移动，Enter/Space 激活；激活项 `aria-current="page"` |
| `AppToast.vue` | 右下角浮出，白底卡片风，成功绿点/失败红点，2.4s 自动消失 | 所有"已保存"反馈统一走 toast，不打断表单 |

### 5.1 通用控件（`src/components/ui/`）

| 组件 | 规格要点 | 状态 |
| :--- | :--- | :--- |
| `AppCard.vue` | 白底、`1.5px --line`、22px 圆角、`padding:18px 20px`、`shadow-card`；标题 13px/900/字距 1，前缀 45° 抹茶菱形 8×8 | — |
| `AppButton.vue` | 高 40、圆角 12、13px/700/字距 1、active `scale(.96)` | `primary`（matcha 底白字+软投影，hover `--matcha-deep`）/ `ghost`（`--bg-deep` 底 + 1.5px 边，hover 边框与文字变绿）/ `danger`（删除确认，红） / `disabled`（`opacity:.45` + `cursor:not-allowed`）/ `loading`（内嵌 14px 旋转圈，文字换「…中」，禁点） |
| `AppToggle.vue` | 46×26 胶囊，滑块 20px 白圆 + 投影，`on` 态 matcha 底 | on / off / disabled（灰底不可点，悬浮解释原因）；受控组件，值变更 emit 后由父级写库 |
| `AppInput.vue` / `AppSelect.vue` | 高 42、圆角 12、`1.5px --line`、白底、padding 0 14px、13.5px；`mono` 变体 Consolas；label 12px/700/`--ink-soft` | default / focus（matcha 边 + 3.5px 外圈）/ error（红边 + 下方 11px 红字）/ readonly（底色 `--bg-deep`）/ password（👁 切换） |
| `SegmentedControl.vue` | 容器 `--bg-deep` 圆角 13 内 padding 4；选项高 34 圆角 10，选中白底 + `--matcha-deep` 文字 + 小投影；支持 `small` 副标 9.5px | single-select；键盘 ←/→ |
| `StatusBadge.vue` | 10.5px/900 胶囊 padding 3×9 | `green`（matcha-soft/matcha-deep）/ `orange`（peach-soft/#E0742C）/ `pink`（#FBE3E7/#D4647A） |
| `HintBox.vue` | `--bg-soft` 底、虚线边、圆角 14、11.5px/1.7 行高，`b` 强调用 `--matcha-deep`；左置 emoji 图标位 | — |
| `KbdChip.vue` | 白底胶囊、`border-bottom:3px`、Consolas 11px/700 | 只读展示快捷键 |
| `TimeChip.vue` | 白底圆角 11、mono 13px/700、内嵌 13px 时钟 SVG（`--matcha-deep`） | hover 出现 ✕；「＋ 添加时段」为 dashed 变体 |

### 5.2 业务组件

| 组件 | 规格 | 数据 |
| :--- | :--- | :--- |
| `ProviderChip.vue` | `.p-chip`：padding 9×16、圆角 12、12.5px/700；选中 `--ink` 深底白字 + 投影；`.dot` 7px 蜜桃橙圆点；★ 必选前缀 | 点击回填 Base URL/模型到所在档位表单 |
| `QuotaBar.vue` | 高 10 圆角 999 轨道 `--bg-deep`；填充 `linear-gradient(90deg, --butter, --peach)`，宽度=剩余比例，宽度变化 400ms 过渡 | 剩余额度（内置额度档） |
| `AffectionRing.vue` | 158×158，`conic-gradient(var(--peach) 0 {angle}, #EDE2CB {angle} 360deg)`，内白圆 `inset:13px`；中心数字 40px 展示体 + `of 1000` 10.5px | props: `points`; angle = points/1000×360；数字滚动动画 |
| `LevelPill.vue` | 深棕（`--ink`）胶囊，17px 展示体白字，字距 2，投影 | `relationship_level` → Lv 文案映射 |
| `LogItem.vue` | 32px 圆角图标位（g=matcha-soft / o=peach-soft / p=#FBE3E7）+ 两行文本 + 右侧 mono delta | `affection_logs` 行 |
| `FactChip.vue` | 白底 999 胶囊 padding 6×8×13、12px；`conf` 9px/900 matcha 微胶囊；19px 圆按钮 ✎/✕ | `facts` 行；编辑=行内受控 input；删除=二次确认态 |
| `DiaryCard.vue` | 暖渐变底（`#FFFDF6→#FBF2DE`）、右上 110px 引号水印、正文 12.5px/2、foot badge + ghost 小按钮（高 30） | `diaries` 行 |
| `PetOptionCard.vue` | 76px 圆头像 + 名称 + 副标；selected：2px matcha 边 + matcha-soft 底 + 「✓ 使用中」角标；dashed 变体为导入占位（`＋` 26px 蜜桃橙） | `active_renderer` 选择 |
| `PreviewStage.vue` | 210px 高、圆角 18、底部抹茶径向光晕；**内嵌 PIXI canvas**（见 §4.1 红线） | `IPetRenderer` 实例持有于组件外普通类属性（**严禁放进 `reactive`/`ref`**），`onUnmounted` 调 `dispose()` + `destroy(true)` 并移除监听（baseline §I/§J） |

### 5.3 组件 → 目录映射

```
src/
├── styles/tokens.css            # 2.1 全部 token（唯一来源，禁在组件写死色值）
├── views/SettingsWindow.vue     # 壳：TitleBar + SideNav + 5 个 panel
└── components/
    ├── window/TitleBar.vue  window/SideNav.vue
    ├── ui/（AppCard / AppButton / AppToggle / AppInput / SegmentedControl /
    │        StatusBadge / HintBox / KbdChip / TimeChip / AppToast）
    ├── settings/（ProviderChip / QuotaBar / PetOptionCard / PreviewStage）
    ├── affection/（AffectionRing / LevelPill / LogItem）
    ├── memory/（DiaryCard / FactChip）
    └── panels/（AvatarPanel / LlmPanel / DndPanel / AffectionPanel / MemoryPanel）
```

---

## 六、全局交互状态规范

| 状态 | 规范 |
| :--- | :--- |
| 加载 | 首次进入面板显示骨架屏（卡片灰条 1.2s 内渲染，禁止布局跳动）；按钮级 loading 见 `AppButton` |
| 保存 | toggle/segmented/chip 类**即改即存**（乐观更新 + 失败回滚 + 红 toast）；表单类（LLM 配置）**显式保存**；统一 toast「已保存」 |
| 错误 | 输入校验：红边 + 11px 红字在字段下方；网络/IPC 错误：红 toast + 卡片顶部错误条（可关闭），**永不弹系统对话框** |
| 空状态 | 每面板定义专属文案（见各 Tab），风格沿用 `HintBox`，可含一个 ghost 行动按钮 |
| 禁用 | 依赖缺失时禁用并悬浮说明（如：未配置 Key 时「测试连接」禁用，悬浮「请先填写 API Key」） |
| 破坏性操作 | 清空 Key / 删除记忆 / 重新生成日记：按钮内二次确认（「确认?」），不做系统 confirm 弹窗 |
| 键盘 | 上一节 nav 键盘规则；表单 Enter 提交；Esc 关闭行内编辑器；焦点环全局可见 |

---

## 七、设计决策依据

| # | 决策 | 依据 |
| :--- | :--- | :--- |
| 1 | **左侧竖排导航而非顶部 Tab** | 5 个板块信息密度差异大（好感度/记忆是"看"，LLM 配置是"填"），竖排导航让分类名始终可见；设计稿已定型且占用 212px 可承受 |
| 2 | **服务商用 chip 组而非下拉框** | 选项 ≤9 个且需要"必选/可选"视觉分组，chip 一次全部可见、单击即切，比 select 少一次点按——直接服务"零门槛"目标（模块 9） |
| 3 | **LLM 配置按 main/memory/fallback 三档分面** | 计划文档「LLM 大模型配置」小节的三档路由是后端既定架构（主对话重质量、沉淀重成本、兜底重可用）；UI 与后端一一对应，避免"一个模型打天下"的体验与成本失衡。复用 `.segmented` 组件，不新增视觉语言 |
| 4 | **Key 保存后只回显脱敏指纹** | `rule.md` 红线"API Key 绝不落前端"；明文回显等于把密钥泄漏给 WebView 内存快照，指纹让用户能确认"已配置"且不破坏红线 |
| 5 | **预览区用真实 PIXI 而非设计稿 jpg** | 设计稿的 `assets/pet-*.jpg` 属展示素材（`development-guide.mdc` FAQ 明确不属运行时资源）；真渲染才能预览果冻物理/表情，且为 Phase 3 `CustomSpriteRenderer` 验证接口替换点 |
| 6 | **好感度圆环用 `conic-gradient` 而非 SVG/Canvas** | 零依赖、纯 CSS、性能好，token 化后主题一致；`<i>` 内层白圆用伪元素实现，无额外 DOM |
| 7 | **衰减流水写成"委屈"叙事** | baseline §F 红线"禁止向用户播报冰冷数值"；看板是给用户自己看的复盘界面，可保留数值，但**文案层必须情绪化**，§4.4 条目即示范 |
| 8 | **自绘标题栏** | 需求要求"关闭仅 hide 不销毁"，系统铬件做不到关闭拦截且视觉与 Jelly-Toy 割裂；已备原生 decorations 回退方案（§5.0） |
| 9 | **字体本地打包** | Tauri WebView 环境不保证外网可达，Google Fonts 会造成首屏字体闪烁或缺失；本地 `@font-face` 是离线桌应用的唯一可靠方案 |
| 10 | **可选低成本服务商默认收起** | 首屏只出现必选两家 + 内置额度 + 自定义（与设计稿 chip 行密度一致），可选组放进展开区——兼顾"零门槛首屏"与"低成本可选项"的完整性 |
| 11 | **即改即存 vs 显式保存的划分** | 单值开关类改了就生效符合直觉且实现简单（一条 update 命令）；LLM 表单含测试连接这一原子动作，必须显式保存避免"半配置"状态触发对话失败 |
| 12 | **窗口尺寸 1000×660** | 设计稿 meta-chip 标注为准；CSS 内 950px 是展示舞台示意宽度，实现时以 Tauri 窗口定义为准并记录进 `CODEBUDDY.md` 配置表 |

---

## 八、验收清单（供 D2/D3 开发与走查）

- [ ] 五个 Tab 布局与设计稿一致性走查（间距 14/18/26、圆角 12/13/18/22、全部色值来自 `tokens.css`，组件内零硬编码色值）
- [ ] 窗口关闭 = 隐藏；托盘「设置」可再次唤起且保留上次 Tab；进程退出仅托盘路径
- [ ] LLM 三档独立保存、互不串档；选中预设自动回填 URL/模型；Key 保存后仅见指纹；测试连接三态（loading/成功 badge/错误行）完整
- [ ] 内置额度耗尽自动切 BYOK 引导；`fallback` 档生效提示可见
- [ ] 好感度圆环与流水真实反映 `pet_states`/`affection_logs`；衰减条目文案为情绪化叙事，无数值播报进对话
- [ ] 记忆编辑/删除后 `vec_memory` 重建成功（后端日志/单测佐证）；删除有二次确认
- [ ] 预览舞台 PIXI 实例在面板切换/窗口隐藏/组件卸载时正确 `dispose`，无多 Ticker、无 reactive 包裹（三项冷眼审查通过）
- [ ] 最小窗口（860×600）与窄于 1080 时布局降级正确，无横向滚动条（内容区除外）
- [ ] 键盘全流程可达；`aria-label` 覆盖全部图标按钮；toast 不抢焦点
- [ ] `pnpm typecheck` / `pnpm lint` / `pnpm lint:rust` 全绿；新增 IPC 命令均已在 `capabilities` 授权
