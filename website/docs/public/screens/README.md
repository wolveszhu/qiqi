# 功能截图采集工作流

本目录存放官网「功能介绍」页使用的真实运行截图。截图由**可运行桌宠应用的 Windows 环境**生成，不在 CI 中自动抓取。

## 采集步骤（在开发机执行）

1. 构建并运行桌面应用：
   ```bash
   pnpm install
   pnpm tauri build --release   # 或直接 pnpm tauri dev 调试
   ```
2. 启动后，依次截取以下界面（建议窗口 1000×660 或 150×150 原比例，导出 webp，单张 < 150KB）：
   - `pet.webp` —— 桌宠层（透明置顶的小窗，展示猫/狗形象与气泡）
   - `settings.webp` —— 设置面板（API 配置 / 勿扰 / 记忆阅读）
   - `chat.webp` —— AI 对话与记忆召回界面
   - `features-*.webp` —— 好感度、主动互动、形象切换等任意可展示界面
3. 将图片放入本目录 `website/public/screens/`。
4. 在 `docs/features.md` / `docs/en/features.md` 的 `<FeatureGrid>` 中，为对应卡片加上
   `image: '/screens/xxx.webp'` 字段即可替换为真实截图。

## 自动化（可选）

若环境与 playwright-cli 可用，可在应用运行后执行脚本批量截图：

```bash
# 伪代码：启动应用 → 定位窗口 → 逐屏截图 → 存入 public/screens/
npx playwright ... 
```

> 注意：截图仅作为官网静态资源副本，不引用应用源码，符合「官网与应用代码层隔离」边界。
