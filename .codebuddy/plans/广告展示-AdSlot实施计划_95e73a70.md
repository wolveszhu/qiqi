---
name: 广告展示-AdSlot实施计划
overview: 为设置窗口（窗口 B）新增本地内置、可关闭的广告模块：① TitleBar 下方的文字型横幅（文案+背景色+跳转链接，无图片）；② 侧边栏底部的图片小卡（300×250 占位素材、显示宽 144px 等比缩放）。关闭行为为"今日不再显示"（经 app_settings 持久化到当天日期，次日自动恢复）。计划确认后的第一个交付物是将本计划完整落地为 .codebuddy/plans/广告展示-AdSlot实施计划.md 文档，随后按文档实施。
design:
  architecture:
    framework: vue
  styleKeywords:
    - 非侵入轻量
    - 果冻暖色
    - Jelly-Toy纸感
  fontSystem:
    fontFamily: Noto Sans SC
    heading:
      size: 13px
      weight: 600
    subheading:
      size: 12px
      weight: 700
    body:
      size: 13px
      weight: 400
  colorSystem:
    primary:
      - "#7CA65B"
      - "#5D8541"
      - "#FFD98A"
    background:
      - "#EAF1DF"
      - "#FFEBDB"
      - "#FFFEF9"
      - "#FBF5EA"
    text:
      - "#403A2E"
      - "#948A75"
      - "#FFFFFF"
    functional:
      - "#EBE1CE"
      - "#FF9E5E"
todos:
  - id: write-plan-doc
    content: 落地计划文档到 .codebuddy/plans/广告展示-AdSlot实施计划.md（含素材规格、数据契约、关闭语义、素材替换指引）
    status: completed
  - id: rust-ad-persistence
    content: app_settings 新增 adDismissals 读写（chrono 写当日日期）与单测，settings 增两命令并注册 lib.rs
    status: completed
  - id: ad-config-asset
    content: 新增 src/config/ads.ts 广告常量与 src/assets/ad/sidebar-300x250.svg 占位素材（明确标注 300×250）
    status: completed
    dependencies:
      - write-plan-doc
  - id: useads-composable
    content: 新增 useAds 单例 composable（本地今日比较/dismiss 失败不隐藏）与 api/settings.ts 两个封装
    status: completed
    dependencies:
      - ad-config-asset
      - rust-ad-persistence
  - id: ad-components
    content: 新增 AdBanner.vue 文字横幅与 AdCard.vue 图片卡（广告标识/关闭/跳转），并补 AdSlot.spec.ts 组件测试
    status: completed
    dependencies:
      - useads-composable
  - id: integrate-settings
    content: SettingsWindow 挂载 AdBanner，SideNav 新增底部 footer slot 承载 AdCard
    status: completed
    dependencies:
      - ad-components
  - id: verify-commit
    content: 全量校验（cargo test/clippy + vitest + typecheck/lint/format）并按 scope 分笔提交
    status: completed
    dependencies:
      - integrate-settings
---

## 产品概述

为设置窗口（窗口 B）新增本地内置的广告展示模块，包含两个广告位：

1. **顶部文字横幅**（TitleBar 下方、所有 Tab 可见）：不用图片，采用「文字 + 背景色 + 跳转链接」形式，单行轻量展示
2. **侧边栏底部图片小卡**（SideNav 导航下方，显示宽 144px）：图片素材规格 **300×250**（IAB 标准 medium rectangle），占位素材先行、等比缩放展示

## 核心特性

- 两个广告位均带「广告」合规标识与「✕」关闭按钮
- 关闭语义为**今日不再显示**：持久化到当天日期，次日自动恢复展示
- 广告数据（文案/图片/链接）为**本地内置常量**，图片打包进应用 assets，随版本更新、无网络依赖
- 点击广告主体经系统浏览器打开跳转链接

## 已确认的边界

- 不在对话面板内嵌广告、不在桌宠窗口 A 展示
- 图片素材先用占位符（明确标注尺寸 300×250），真实素材只需同路径同规格替换
- 非图片广告位（顶部横幅）不涉及素材规格

## 技术栈

- 前端：Vue 3 `<script setup lang="ts">` + 项目自建 UI 组件（AppCard/StatusBadge 等）+ Jelly-Toy 设计 token（`src/styles/tokens.css` 唯一样式真源，禁止硬编码色值）
- 后端：Tauri 2 + Rust；关闭状态复用 `app_settings` type=3 交互开关桶（`merge_interaction_value` 合并写入，与 `snapSleepEnabled` 同桶互不干扰）
- 跳转：`@tauri-apps/plugin-opener` 的 `openUrl`（`opener:allow-open-url` 已授权）
- 日期：Rust 侧用 `chrono::Local` 写入当日日期（权威时区源）；前端用本地时区格式化 `YYYY-MM-DD` 比较（避开 `toISOString()` 的 UTC 偏移坑）

## 实现方案

- **数据契约**：关闭状态存为 type=3 桶内 `adDismissals` 嵌套映射 `{ [素材ID]: "YYYY-MM-DD" }`；可见性 = `dismissals[id] !== 本地今日`（昨日及更早条目自然失效，ID 集合固定、无需清理）
- **单一真源**：广告内容为前端常量模块（素材 ID/文案/图片/链接）；关闭状态为后端持久化（Rust 写日期，避免 JS/Rust 时区分歧）
- **组件边界**：`SideNav` 是零业务 UI 组件——通过新增**底部 footer slot** 注入广告卡（组件自身保持纯 UI）；顶部横幅仿 `UpdateNotice.vue` 先例挂在 SettingsWindow
- **失败语义**：关闭持久化失败 → toast 提示且广告保持展示（「今日不再显示」承诺未兑现不静默骗人）；加载失败 → 回落全部可见

### 关键代码结构

```ts
// src/config/ads.ts — 广告位定义（本地内置单一真源）
export interface AdItem {
  id: string;            // 素材唯一 ID（关闭持久化的键；换素材必须换 ID）
  slot: "top-banner" | "sidebar-card";
  text?: string;         // 文字横幅文案（top-banner）
  tone?: "matcha" | "peach"; // 横幅背景色 token
  image?: string;        // 图片 URL（sidebar-card；素材规格 300×250）
  link: string;          // 点击跳转链接
}
```

```rust
// app_settings.rs — 关闭状态读写（Rust 写入当日日期）
pub const KEY_AD_DISMISSALS: &str = "adDismissals";
pub fn get_ad_dismissals(conn: &Connection) -> Result<HashMap<String, String>, AppError>;
pub fn mark_ad_dismissed(conn: &Connection, ad_id: &str) -> Result<(), AppError>;
```

## 目录结构

```
.codebuddy/plans/广告展示-AdSlot实施计划.md  # [NEW] 计划文档交付物（用户显式要求）
src-tauri/src/db/repositories/app_settings.rs  # [MODIFY] adDismissals 读写 + 单测（roundtrip/默认空/合并互扰）
src-tauri/src/commands/settings.rs             # [MODIFY] settings_get_ad_dismissals / settings_mark_ad_dismissed
src-tauri/src/lib.rs                           # [MODIFY] invoke_handler 注册
src/config/ads.ts                              # [NEW] 广告位常量（2 条：顶部横幅 + 侧边栏卡）
src/assets/ad/sidebar-300x250.png              # [NEW] 300×250 占位素材（替换真图时同路径同规格 PNG）
src/composables/useAds.ts                      # [NEW] 单例 composable：加载关闭映射 + 本地今日比较 + dismiss
src/api/settings.ts                            # [MODIFY] getAdDismissals / markAdDismissed 封装（走 callCommand）
src/components/settings/AdBanner.vue           # [NEW] 顶部文字横幅（标识+文案+链接+关闭）
src/components/settings/AdCard.vue             # [NEW] 侧边栏图片卡（300×250→144×120 等比 + 角标 + 关闭）
src/components/window/SideNav.vue              # [MODIFY] 导航后追加底部 footer slot（margin-top:auto）
src/views/settings/SettingsWindow.vue          # [MODIFY] TitleBar 下挂载 AdBanner
src/tests/components/AdSlot.spec.ts            # [NEW] 组件测试（渲染/关闭/跳转，仿 ChatInput.spec 模式）
```

## 设计风格

延续 Jelly-Toy 暖奶油纸感，广告作为**非侵入式轻量单元**融入现有布局：

**① 顶部文字横幅（AdBanner）**：位于 TitleBar 之下（与 UpdateNotice 同层级堆叠，UpdateNotice 优先显示）、内容区之上通栏。单行结构：左端「广告」小标签（butter 黄底 #FFD98A、ink 深字、圆角胶囊）+ 中部文案（13px，--ink 暖炭棕）+ 右侧「了解详情 →」链接（--matcha-deep 抹茶深绿，hover 加深下划线）+ ✕ 关闭按钮（--ink-soft，hover 浮起）。背景按素材 tone 取 --matcha-soft（#EAF1DF）或 --peach-soft（#FFEBDB），细描边 --line，无阴影（区别于功能卡片，弱化存在感），高度约 36px，出现/消失带 --dur-fast 滑落过渡。

**② 侧边栏底部图片卡（AdCard）**：SideNav 底部（footer slot，margin-top:auto 贴底），宽 144px；300×250 素材等比缩放为 144×120，圆角 --r-sm、描边 --line、hover 轻微上浮 + 阴影 --shadow-card；左上角「广告」角标（半透明 ink 底白字 10px）；右上角 ✕ 关闭（悬停显影）；整卡可点击经 openUrl 跳转。与上方导航项间距 12px。

**合规**：两个广告位均带「广告」标识；关闭按钮明确可点（≥16px 命中区）。