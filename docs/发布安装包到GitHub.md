# 发布安装包到 GitHub（手动推送教程）

> 面向 Windows 安装包 `msi` / `nsis.exe` 的分发。最近核对：2026-11-07。
> 仓库：`wolveszhu/qiqi`（public）、默认分支 `main`、remote `origin = git@github-qiqi:wolveszhu/qiqi.git`。

---

## 0. 当前状态（先看这里）

| 项 | 状态 |
| :--- | :--- |
| `release/` 里的安装包 | 已存在，`qiqi_0.1.0_x64_en-US.msi`（7,356,416 B）、`qiqi_0.1.0_x64-setup.exe`（5,587,819 B） |
| 是否已推送到 GitHub | **已推送**。提交 `592455c feat(release): 新增 0.1.0 版本 Windows 安装包（NSIS 与 MSI）` 已在远端 `main` |
| 匿名下载直链 | **可用**（HEAD 实测 200），见 §6 |
| GitHub Release | **尚无任何 Release**（公开 API 返回空数组） |
| tag `v0.1.0` | 已存在，但指向 `08a7564`（发布工作流配置那笔），**早于**安装包提交 |
| 本机工具 | 无 `gh` CLI、无 `GITHUB_TOKEN`；只有 SSH deploy key（`~/.ssh/qiqi_deploy`），**只能 git push，不能调 API 建 Release** |

**结论**：现在别人已经能从仓库直链下载；但下载体验更好的是 GitHub Release 页面（有版本号、更新日志、Assets 下载按钮）。若只想要一个"能下载的链接"，§6 已够；若想要正式的发布页，按 §3 手动操作（推荐）。

---

## 1. 三种分发方式怎么选

| 方式 | 适合 | 优点 | 缺点 |
| :--- | :--- | :--- | :--- |
| A. 仓库文件（当前做法） | 临时给个链接 | 零配置，`git push` 即可 | 二进制进 git 仓库会持续膨胀；没有版本页/更新日志；`.gitignore` 已忽略 `release/`，每次必须 `git add -f` |
| **B. GitHub Release 资产（推荐）** | 正式对外发布 | 版本清晰、有更新日志与 Assets 下载区、可标 Latest、可传校验文件；**不污染 git 仓库** | 需要网页或 `gh` 手动上传（本机缺凭据） |
| C. 打 tag 触发 Actions 自动发布 | 长期迭代 | 全自动，`release.yml` 已配好 | 首次要等 CI 编译 10–20 min；`releaseDraft: true` 出的是**草稿**，必须手动 Publish；`v0.1.0` tag 已占用，需先提升版本号 |

> 建议：**以后走 B（或 C），别再往 git 里塞二进制**。仓库里已有的两个文件不影响使用，是否清理见 §8。

---

## 2. 方式 A：更新仓库里的安装包（当前做法）

本地出包（会自动跑发布数据守卫）：

```powershell
cd d:\qiqi\qiqi
.\scripts\build-release.cmd
```

产物收集到 `release/`。因为 `.gitignore` 有 `release/`，**必须强制 add**：

```powershell
git add -f release/qiqi_0.1.0_x64_en-US.msi release/qiqi_0.1.0_x64-setup.exe
git commit -m "feat(release): 更新 0.1.0 版本 Windows 安装包"
git push origin main
```

推送后校验：

```powershell
git ls-remote origin refs/heads/main   # 应与本地 HEAD 一致
```

---

## 3. 方式 B（推荐）：网页手动创建 Release 并上传资产

全程浏览器操作，不需要装任何工具、不需要 token。

1. 打开 **新建 Release 页**：<https://github.com/wolveszhu/qiqi/releases/new>
   （或：仓库首页 → 右侧 `Releases` → `Draft a new release`）

2. **选择 tag**：点 `Choose a tag` 下拉，选已存在的 `v0.1.0`。
   - 目标分支选 `main`。
   - 若想和最新代码严格对应，可新建 `v0.1.1`（但记得同步改 `src-tauri/tauri.conf.json` 的 `version`），见 §7。

3. **标题**：`Qiqi v0.1.0`（或 `七七 v0.1.0`）。

4. **说明**：可直接粘贴下面这段（含校验和，方便用户验证下载完整性）：

   ````markdown
   ## 七七 v0.1.0 · Windows 安装包

   - `qiqi_0.1.0_x64_en-US.msi` — 系统级安装 / 卸载（推荐，7.02 MiB）
   - `qiqi_0.1.0_x64-setup.exe` — NSIS 安装向导（5.33 MiB）

   ### 系统要求

   Windows 10 / 11，且已安装 WebView2 运行时（Windows 11 自带；Windows 10 首次运行会引导安装）。

   ### 说明

   安装包未签名，SmartScreen 可能拦截，选择「仍要运行」即可。
   关闭设置窗口只是隐藏桌宠，退出请走系统托盘 → 「退出」。

   ### SHA256

   ```
   B1D21B8FFD8A04584BB48DC9B9F0759995B7FC25DB67F4658FAAE1E8FD922A4F  qiqi_0.1.0_x64_en-US.msi
   53892BC7D1E1B0AF413A6C3C2D152F1DAB9BB3DF5AA115D7C93555075151000E  qiqi_0.1.0_x64-setup.exe
   ```
   ````

5. **上传资产**：把 `d:\qiqi\qiqi\release\` 下的两个文件拖到页面下方的
   `Attach binaries by dropping them here or selecting them`（或直接点 `selecting them` 选文件）。
   等两个文件都显示打勾 / 进度条走完再继续。

6. **发布选项**：
   - 勾 `Set as the latest release`（作为最新版展示）。
   - `Create a draft` **不要勾**；如果勾了存成草稿，别人看不到。
   - `This is a pre-release` 不勾（除非是测试版）。

7. 点 **`Publish release`**。

8. **验证**（重要）：
   - 打开 <https://github.com/wolveszhu/qiqi/releases>，应能看到 `Qiqi v0.1.0` 与 `Latest` 徽标，Assets 展开有 2 个文件。
   - 用**无痕窗口**（未登录 GitHub）点一下下载链接，确认匿名可下。
   - 若页面显示 `Draft`，说明第 6 步存成了草稿：进该 Release → `Edit` → `Publish release`。

> 注意：`release.yml` 里 `releaseDraft: true` 只影响 **Actions 自动发布**；网页手动发布不受它影响。

---

## 4. 方式 B 的命令行版（可选，需装 `gh`）

本机当前**未安装** `gh`，如想用命令行，先安装并登录一次：

```powershell
winget install --id GitHub.cli
gh auth login        # 选 GitHub.com → HTTPS → 浏览器授权
```

创建 Release 并上传资产（`v0.1.0` tag 已存在，`--verify-tag` 校验它确实存在）：

```powershell
cd d:\qiqi\qiqi
gh release create v0.1.0 `
  --title "Qiqi v0.1.0" `
  --notes-file release-notes.md `
  --verify-tag `
  .\release\qiqi_0.1.0_x64_en-US.msi `
  .\release\qiqi_0.1.0_x64-setup.exe
```

追加 / 替换资产：

```powershell
gh release upload v0.1.0 .\release\qiqi_0.1.0_x64_en-US.msi --clobber
gh release view v0.1.0 --web      # 打开网页核对
```

> 若 tag 不存在，去掉 `--verify-tag`，`gh` 会自动基于 `main` 创建 tag。

---

## 5. 方式 C：打 tag 触发 GitHub Actions 自动发布

工作流：`.github/workflows/release.yml`，推送 `v*` tag 时自动构建 msi + nsis 并创建 Release。

```powershell
# 1) 必须先提升版本号（v0.1.0 tag 已存在，不能重复打）
#    编辑 src-tauri/tauri.conf.json 的 "version": "0.1.1"
#    同时建议同步 package.json（当前 "version": "1.0"，与 Tauri 版本不一致）
git add src-tauri/tauri.conf.json
git commit -m "chore(release): 提升版本到 0.1.1"
git push origin main

# 2) 打 tag 触发
git tag v0.1.1
git push origin v0.1.1
```

然后：

1. 看进度：<https://github.com/wolveszhu/qiqi/actions>
2. 成功后到 <https://github.com/wolveszhu/qiqi/releases>，会看到 **`Qiqi v0.1.1` 草稿**。
3. **必须手动点 `Publish`**，否则用户看不到（工作流设了 `releaseDraft: true`）。

也可在 Actions 页面用 `workflow_dispatch` 手动跑一次调试，但那不会创建 Release 资产。

---

## 6. 下载链接（可直接用）

**仓库文件直链（当前可用，已实测 200）**

- MSI：<https://github.com/wolveszhu/qiqi/raw/main/release/qiqi_0.1.0_x64_en-US.msi>
- NSIS：<https://github.com/wolveszhu/qiqi/raw/main/release/qiqi_0.1.0_x64-setup.exe>

等价的 raw 域名形式：`https://raw.githubusercontent.com/wolveszhu/qiqi/main/release/<文件名>`

**Release 资产链接（按 §3 发布后才生效）**

- `https://github.com/wolveszhu/qiqi/releases/download/v0.1.0/qiqi_0.1.0_x64_en-US.msi`
- `https://github.com/wolveszhu/qiqi/releases/download/v0.1.0/qiqi_0.1.0_x64-setup.exe`

**总入口**：<https://github.com/wolveszhu/qiqi/releases>

**下载后校验**（PowerShell）：

```powershell
cd $env:USERPROFILE\Downloads
Get-FileHash .\qiqi_0.1.0_x64_en-US.msi -Algorithm SHA256
# 应为 B1D21B8FFD8A04584BB48DC9B9F0759995B7FC25DB67F4658FAAE1E8FD922A4F
Get-FileHash .\qiqi_0.1.0_x64-setup.exe -Algorithm SHA256
# 应为 53892BC7D1E1B0AF413A6C3C2D152F1DAB9BB3DF5AA115D7C93555075151000E
```

---

## 7. 每次发新版本的清单

1. 改 `src-tauri/tauri.conf.json` 的 `version`（如 `0.1.0` → `0.1.1`）。
2. 跑 `.\scripts\build-release.cmd` 出包（会自动跑发布数据守卫，脏数据会中止）。
3. 核对 `release/` 里两个文件名带新版本号。
4. 按 §3 新建 Release（新 tag）或按 §5 打 tag 走 CI。
5. 重算 SHA256 并写进 Release 说明：

   ```powershell
   Get-ChildItem .\release -File | Get-FileHash -Algorithm SHA256 |
     ForEach-Object { "$($_.Hash)  $(Split-Path $_.Path -Leaf)" }
   ```

6. 无痕窗口验证匿名可下载；确认不是 `Draft`。
7. 若走 CI，记得把草稿 `Publish`。

---

## 8. 常见问题

| 现象 | 原因 / 处理 |
| :--- | :--- |
| 用户打不开下载链接 | 确认仓库 public（当前是）；若 Release 是 `Draft`，未登录用户看不到 → `Publish release` |
| `git add release/...` 没反应 | `.gitignore` 忽略了 `release/`，必须用 `git add -f` |
| 想删掉仓库里的二进制 | 不建议擅自删已提交文件。若确要清理：`git rm --cached release/*.msi release/*.exe` 后提交，但**历史提交里仍保留 blob**，仓库体积不会立刻变小；彻底清理需 `git filter-repo`（会改写历史，谨慎） |
| `git push origin v0.1.0` 报 tag 已存在 | `v0.1.0` 已推送。提升版本号打新 tag，不要删旧 tag |
| `tauri.conf.json` 与 `package.json` 版本不一致 | 当前分别是 `0.1.0` / `1.0`。Tauri 打包只认前者；建议后续统一，避免混淆 |
| 安装被 SmartScreen 拦 | 安装包未签名，选「仍要运行」；长期方案是申请代码签名证书 |
| Windows 10 装完打不开 | 缺 WebView2 运行时，安装微软 Evergreen Runtime 即可（Win11 自带） |
| 装完还有旧数据 / 记忆 | MSI 重装不清 `%APPDATA%\com.brave.qiqi\pet.db`。全新验收需先退出应用并删该目录，见 `src-tauri/src/db/README.md` §7.4 |
