# 七七官网部署流程

本文档说明如何将官网（`website/` 下的 VitePress 工程）构建为**纯静态站点**并部署到线上域名 **`qiqi.bootai.work`**。

> 核心结论：官网构建后是标准静态文件（HTML/CSS/JS/图片），**不依赖任何运行时环境**，解压到任意静态主机即可访问。

---

## 1. 产物说明

| 名称 | 位置 | 说明 |
|---|---|---|
| 源码工程 | `website/` | VitePress 工程，仅构建时使用 |
| 站点配置 | `website/.vitepress/config.ts` | `srcDir: "docs"`、`cleanUrls: true`、双语、`base` 默认 `/` |
| 静态资源 | `website/docs/public/` | `logo.png`、`og.png`、`screens/`、`CNAME`（自定义域名文件，`latest.json` 由脚本在构建时生成） |
| 构建输出 | `website/.vitepress/dist` | 最终可部署的静态文件 |
| 版本同步 | `website/scripts/sync-latest.mjs` | 构建时自动把根目录 `latest.json` 拷入资源目录，供下载页显示版本号 |

`CNAME` 文件内容为：

```
qiqi.bootai.work
```

它会在构建时自动复制到 `dist/CNAME`，用于告知 GitHub Pages 绑定自定义域名。

---

## 2. 本地构建（生成部署包）

> 只有「生成部署包」这一步需要 Node.js 环境；部署目标主机不需要。

### 前置要求
- Node.js ≥ 18
- 包管理器 `pnpm`（推荐，CI 同款）或 `npm`

### 步骤
```bash
cd website
pnpm install            # 或 npm install
pnpm build              # = node scripts/sync-latest.mjs && vitepress build .
```

构建完成后，静态文件位于 `website/.vitepress/dist`，其根目录应包含：

```
dist/
├── index.html          # 中文首页
├── en/index.html       # 英文首页
├── features.html       # 功能页（cleanUrls，见 §4 注意）
├── download.html
├── logo.png
├── og.png
├── latest.json
├── CNAME               # 自定义域名文件
├── assets/             # JS/CSS/字体
├── screens/            # 功能页截图
└── ...
```

### 直接获取已构建包
仓库根目录已生成一次完整包：`qiqi-bootai-work-dist.zip`（约 3MB），解压即用，等同于上面的 `dist/` 内容。需要重新生成时按上面命令构建后自行压缩 `dist/` 即可。

---

## 3. 部署方式一：GitHub Pages + 自定义域名（推荐）

适合已有 GitHub 仓库、希望 CI 自动部署的场景。域名 `qiqi.bootai.work` 通过仓库设置绑定。

### 3.1 DNS 配置（域名服务商处）
在 `qiqi.bootai.work` 的 DNS 中添加一条 **CNAME 记录**，指向你的 GitHub Pages 地址，例如：

```
qiqi.bootai.work  CNAME  wolveszhu.github.io
```

> 若用 A 记录，请按 GitHub 官方当前 IP（185.199.108.153 等 4 个）配置。以 GitHub 文档为准。

### 3.2 仓库设置
1. 进入仓库 **Settings → Pages → Build and deployment → Source**，选择 **GitHub Actions**。
2. 在 **Custom domain** 填写 `qiqi.bootai.work`，保存（会自动校验并写入/确认 `CNAME`）。

### 3.3 工作流文件
在仓库创建 `.github/workflows/deploy.yml`：

```yaml
name: Deploy Website
on:
  push:
    branches: [main]
    paths:
      - "website/**"
      - "latest.json"
      - ".github/workflows/deploy.yml"
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: website
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
          cache-dependency-path: website/pnpm-lock.yaml
      - run: pnpm install --frozen-lockfile
      - run: pnpm build
      - uses: actions/upload-pages-artifact@v3
        with:
          path: website/.vitepress/dist
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

推送 `main` 后，Actions 会自动构建并发布；`CNAME` 文件已内置，Pages 将绑定 `qiqi.bootai.work`。

---

## 4. 部署方式二：任意静态主机 / 自有 Nginx

适合放到自己的服务器、对象存储（S3/OSS/COS）、Netlify、Vercel 等。把 `dist/` 内容上传到站点根目录即可。

### 4.1 Nginx 示例
由于开启了 `cleanUrls: true`，链接形如 `/features`（无 `.html`）。Nginx 需加一条回退规则把无后缀路径映射到对应 `.html`：

```nginx
server {
    listen 80;
    server_name qiqi.bootai.work;

    root /var/www/qiqi;        # 上传 dist/ 内容的目录
    index index.html;

    # cleanUrls：/features -> /features.html
    location / {
        try_files $uri $uri.html $uri/ =404;
    }

    # 静态资源长缓存
    location ~* \.(js|css|png|jpg|webp|svg|woff2?)$ {
        expires 30d;
        add_header Cache-Control "public, immutable";
    }

    # 强制 HTTPS（配合证书后开启）
    # listen 443 ssl; ...
}
```

> GitHub Pages / Netlify / Vercel 已内置「pretty URL」映射，无需额外配置；仅**纯 Nginx 且未配 rewrite** 时才需上面的 `try_files`。

### 4.2 对象存储 / CDN
将 `dist/` 全部文件上传到 Bucket 根路径，开启「静态网站托管」并绑定自定义域名 `qiqi.bootai.work`，默认索引文件设为 `index.html` 即可。

---

## 5. 部署后验证清单

- [ ] 访问 `https://qiqi.bootai.work/` 显示中文首页，`/en/` 显示英文首页
- [ ] 浏览器标签页图标为吉祥物 `logo.png`，社交分享图 `og.png` 正常
- [ ] 导航栏「功能 / 下载 / 更新日志 / 常见问题」均可打开，且 `/features` 能直接访问（验证 cleanUrls）
- [ ] 下载页能读取到版本号（来自 `latest.json`，无需联网回源）
- [ ] 语言切换（`/zh` ↔ `/en`）正常
- [ ] 移动端窄屏下功能卡片自动单列
- [ ] DNS 解析已生效，HTTPS 证书有效

---

## 6. 日常更新流程

1. 修改 `website/docs/` 下的页面内容（Markdown）或 `website/.vitepress/` 下的主题/配置。
2. 本地 `pnpm build` 自测（可选）：
   ```bash
   cd website && pnpm build && pnpm preview
   ```
3. 提交并推送 `main`：
   - 若用 §3 工作流：自动重新部署。
   - 若手动部署：重新打包 `dist/` 并覆盖上传到主机。
4. 更新版本号时，修改仓库根 `latest.json`（下载页读取此文件）。

---

## 7. 注意事项

- **运行期外部依赖（非阻断）**：
  - 玩具字体来自 Google Fonts CDN；无网时自动回退系统字体，不影响内容与布局。
  - 下载页「前往 GitHub Releases 下载」为外链；版本号 `latest.json` 已内置于包内，无需回源。
- **`cleanUrls`**：见 §4.1，纯 Nginx 部署务必加 `try_files`。
- **CNAME 文件**：由 `website/docs/public/CNAME` 提供，勿在 `dist/` 手动删除，否则 GitHub Pages 自定义域名会失效。
- **资源隔离边界**：所有站点图片均为 `website/docs/public/` 下的副本，不引用应用源码，符合「官网与代码层隔离」原则。
- **不要提交构建产物**：`.vitepress/dist`、`latest.json`（生成副本）、打包 zip 均属临时产物，无需纳入版本库。
