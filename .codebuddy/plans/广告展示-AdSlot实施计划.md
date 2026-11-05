# 广告展示（AdSlot）实施计划

> 状态：已确认，实施中（2026-11-04）；素材规格于 2026-11-05 修订为 6:5 位图。
> 适用范围：窗口 B（设置层）广告模块。样式遵循 `src/styles/tokens.css`（Jelly-Toy）唯一真源。

## 1. 需求与已确认边界

| 项 | 决策 |
| :--- | :--- |
| 广告位 ① | 设置窗顶部横幅（TitleBar 下方、所有 Tab 可见）——**不用图片**：文字 + 背景色 + 跳转链接 |
| 广告位 ② | 侧边栏底部图片小卡（SideNav 导航下方，显示宽 144px） |
| 素材规格 | 位图 **PNG / JPG / JPEG**，宽高比 **6:5**，**不限制像素尺寸**；不使用 SVG（生成/维护麻烦） |
| 数据来源 | **本地内置**：文案/链接为前端常量（`src/config/ads.ts`），图片打包进 `src/assets/ad/`，随版本更新、无网络依赖 |
| 关闭行为 | **今日不再显示**：持久化到当天日期，次日自动恢复；两个广告位独立关闭 |
| 跳转 | 点击经 `@tauri-apps/plugin-opener` 的 `openUrl` 打开系统浏览器（权限已授权） |
| 合规 | 两个广告位均带「广告」标识 + 明确可点的「✕」关闭（命中区 ≥16px） |
| 不做 | 不在对话面板内嵌、不在桌宠窗口 A 展示、不接远端运营接口 |

## 2. 布局与素材规格

窗口 B 为 1000×660：自绘 TitleBar + SideNav（168px）+ 内容区（padding 24px 28px，可用宽约 776px）。

### 广告位 ① 顶部文字横幅（AdBanner）
- 位置：TitleBar 之下、内容区之上通栏；与 `UpdateNotice` 同层级堆叠（更新提醒优先）。
- 结构：左「广告」胶囊标签（`--butter` 底）＋ 中部文案（13px `--ink`）＋ 右「了解详情 →」（`--matcha-deep`）＋ 「✕」关闭。
- 背景：按素材 `tone` 取 `--matcha-soft` 或 `--peach-soft`，细描边 `--line`、**无阴影**（弱化存在感），高度约 36px。
- **无图片**，不涉及素材规格；出现/消失带 `--dur-fast` 滑落过渡。

### 广告位 ② 侧边栏图片卡（AdCard）
- 位置：SideNav 通过新增**底部 footer slot** 注入（组件保持纯 UI，零业务依赖），`margin-top:auto` 贴底，与导航项间距 12px。
- **素材规格：宽高比 `6:5`** —— 位图格式 PNG / JPG / JPEG，**不限制像素尺寸**（300×250、720×600、861×710 等只要 6:5 均可）；不再使用 SVG 占位图。
- 当前素材：`src/assets/ad/sidebar-300x250.png`（阿里云推广图，861×710 ≈ 6:5）。**文件名沿用历史命名，仅作标识，不代表要求 300×250 像素**。
- 显示尺寸：宽 144px → **144×120 px**（正好 6:5）；`object-fit: cover` 居中裁切，任意合规素材都不会拉伸变形；圆角 `--r-sm`、描边 `--line`、hover 上浮 + `--shadow-card`。
- 左上角「广告」角标（半透明 ink 底白字 10px）；右上角「✕」关闭（悬停显影）；整卡可点击跳转。

### 替换素材指引
1. 准备宽高比 **6:5** 的位图（PNG / JPG / JPEG），像素尺寸不限（建议短边 ≥ 500px 保证清晰）；**不要再用 SVG**。
2. 直接覆盖 `src/assets/ad/sidebar-300x250.png` 即可（文件名沿用历史命名）；若改了文件名，需同步修改 `src/config/ads.ts` 的 `image` 引用与 `AdSlot.spec.ts` 中的断言。
3. 若换的是新一轮广告活动，**必须同时更换素材 `id`**——关闭状态按素材 ID 记录，换 ID 才会重新展示（沿用旧 ID 会被上次「今日不再显示」抑制）。
4. 旧 SVG 占位素材已从版本库移除，不再引入。

## 3. 数据契约

### 3.1 广告内容（前端单一真源）
```ts
// src/config/ads.ts
export interface AdItem {
  id: string;                          // 素材唯一 ID（关闭持久化的键；换素材必换 ID）
  slot: "top-banner" | "sidebar-card";
  text?: string;                       // 文字横幅文案（top-banner）
  tone?: "matcha" | "peach";           // 横幅背景色 token
  image?: string;                      // 图片 URL（sidebar-card；6:5 位图 PNG/JPG/JPEG）
  link: string;                        // 点击跳转链接
}
```

### 3.2 关闭状态（后端持久化，SQLCipher）
复用 `app_settings` **type=3 交互开关桶**（`merge_interaction_value` 合并写入，与 `snapSleepEnabled`/`snapShowRatio` 同桶互不干扰）：

```json
{
  "snapSleepEnabled": true,
  "adDismissals": { "top-banner-2026-11": "2026-11-04", "sidebar-2026-11": "2026-11-04" }
}
```

- `adDismissals`：`{ [素材ID]: "YYYY-MM-DD" }`；**昨日及更早条目自然失效**，ID 集合固定无需清理。
- 可见性判定：`dismissals[id] !== 本地今日`。
- Rust 用 `chrono::Local` 写入当日日期（权威时区源）；前端用本地时区格式化 `YYYY-MM-DD` 比较（避开 `toISOString()` 的 UTC 偏移坑）。

### 3.3 命令（自定义命令免 capability 授权）
| 命令 | 入参 | 返回 | 说明 |
| :--- | :--- | :--- | :--- |
| `settings_get_ad_dismissals` | — | `Record<string,string>` | 读关闭映射（缺省 `{}`） |
| `settings_mark_ad_dismissed` | `adId: string` | `()` | 写当日日期（校验非空 ID） |

## 4. 失败语义

- 关闭持久化失败 → toast 提示且**广告保持展示**（不静默假装已关闭）。
- 关闭映射加载失败 → 回落全部可见（广告是可选增强，不阻断设置窗）。
- 跳转失败 → `AppToast` 提示，不崩溃。

## 5. 目录结构

```
.codebuddy/plans/广告展示-AdSlot实施计划.md  # [NEW] 本文档
src-tauri/src/db/repositories/app_settings.rs  # [MODIFY] adDismissals 读写 + 单测
src-tauri/src/commands/settings.rs             # [MODIFY] 两个命令
src-tauri/src/lib.rs                           # [MODIFY] invoke_handler 注册
src/config/ads.ts                              # [NEW] 广告位常量（2 条）
src/assets/ad/sidebar-300x250.png              # [NEW] 6:5 位图素材（PNG/JPG/JPEG）
src/composables/useAds.ts                      # [NEW] 单例 composable
src/api/settings.ts                            # [MODIFY] 两个 IPC 封装
src/components/settings/AdBanner.vue           # [NEW] 顶部文字横幅
src/components/settings/AdCard.vue             # [NEW] 侧边栏图片卡
src/components/window/SideNav.vue              # [MODIFY] 底部 footer slot
src/views/settings/SettingsWindow.vue          # [MODIFY] 挂载 AdBanner
src/tests/components/AdSlot.spec.ts            # [NEW] 组件测试
```

## 6. 实施步骤

1. 落地本文档（用户显式要求）。
2. Rust 持久化：`app_settings` 增 `KEY_AD_DISMISSALS`、`get_ad_dismissals`、`mark_ad_dismissed` + 单测（roundtrip / 缺省空 / 同桶合并互扰）；`commands/settings.rs` 增两命令；`lib.rs` 注册。
3. 前端本地内置：`src/config/ads.ts` + `src/assets/ad/sidebar-300x250.png` 位图素材（6:5）。
4. `useAds` 单例 composable：加载关闭映射 + 本地今日比较 + `dismiss`；`api/settings.ts` 两封装（统一走 `callCommand`）。
5. 组件：`AdBanner.vue`（文字型）/ `AdCard.vue`（图片型）+ `AdSlot.spec.ts`。
6. 集成：`SettingsWindow` 挂 `AdBanner`；`SideNav` 加 footer slot 承载 `AdCard`。
7. 校验：`cargo test`/clippy、vitest、`typecheck`/lint/format；按 scope 分笔提交。

## 7. 验证

- 后端：`cargo test --lib ad`（含跨天失效与同桶合并互扰）、`cargo clippy`。
- 前端：`pnpm typecheck`、`pnpm test`（AdSlot 组件测试）、`pnpm lint`、`pnpm format:check`。
- 手工：`pnpm tauri dev` → 设置窗顶部出现文字横幅、侧边栏底部出现 6:5（144×120）缩放卡；点「✕」后本日内不再出现，关闭重开仍不出现；手动把系统日期改到次日（或改 DB 中日期）后恢复；点击广告主体打开系统浏览器。
