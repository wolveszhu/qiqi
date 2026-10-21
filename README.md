# qiqi · 桌面数字生命（PC 桌宠）

住在你桌面上的数字生命：有性格、有记忆、有主动性。基于 **Tauri 2 + Vue 3 + TypeScript + PIXI.js**，采用「Rust 系统外壳 + WebView 表现层」的双进程架构，用户数据全部经 SQLCipher 本地加密。

## 快速开始

```bash
pnpm install
pnpm tauri dev      # 完整桌面应用：Rust + 前端热重载
```

| 目的 | 命令 |
| :--- | :--- |
| 仅调试 UI（Vite，http://localhost:1420） | `pnpm dev` |
| 前端构建（含类型检查） | `pnpm build` |
| 预览前端产物 | `pnpm preview` |
| 打包安装包 | `pnpm tauri build` |

需要 Rust 工具链与 WebView2（Windows 目标平台）。

## 代码检查

| 命令 | 说明 |
| :--- | :--- |
| `pnpm typecheck` | `vue-tsc --noEmit` 类型检查 |
| `pnpm lint` / `pnpm lint:fix` | ESLint 检查 / 自动修复 |
| `pnpm format` / `pnpm format:check` | Prettier 格式化 / 校验 |
| `pnpm lint:rust` | `cargo fmt --check` + `cargo clippy` |

> ESLint / Prettier 依赖尚未写入 `package.json`。首次使用前请先安装，命令见 `.codebuddy/rules/lint-rules.mdc` 第 2 节。

## 文档

- **[CODEBUDDY.md](CODEBUDDY.md)** — 命令、架构、集中配置管理、文档规范
- `docs/design/桌宠.md` — 产品与技术规格 v2.0
- `docs/design/rule.md` — 开发规则（Tauri 2 / Rust / PIXI / 数据库迁移）
- `docs/design/pet-design/index.html` — 设置窗口设计稿与 Jelly-Toy 设计令牌
- `.codebuddy/rules/*.mdc` — 代码规范、lint 规则、开发指南、设计基线快照、自动提交纪律

规范、lint 配置与提交约束以 `.codebuddy/rules/` 下的规则文件为准。

## 推荐 IDE

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
