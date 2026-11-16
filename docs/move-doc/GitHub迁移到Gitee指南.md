# 七七：GitHub → Gitee 迁移指南

> 版本：v1.0 · 2026-11-16
> 适用版本：v0.1.1（tag `v0.1.1` → `ebe67b8`）
> 目标：解决国内访问仓库、下载安装包慢的问题。
> 配套文件：[迁移文件清单.md](./迁移文件清单.md)（逐文件、逐行的替换清单）

本文只描述迁移方法与步骤，**未改动仓库现状**。文中 `<owner>` 请替换为你的 Gitee 用户名（若沿用 `wolveszhu`，需先在 Gitee 确认该用户名可用）。

---

## 0. 先选方案（最重要的一步）

迁移的最大约束不是代码，而是**安装包构建**：当前 `.github/workflows/release.yml` 用 `tauri-apps/tauri-action@v0` 在 `windows-latest` runner 上产出 MSI + NSIS，Gitee 侧没有等价的托管动作，Windows runner 的可用性需以 Gitee 当前产品能力为准。

| 方案 | 做法 | 优点 | 代价 | 适用 |
| :--- | :--- | :--- | :--- | :--- |
| **A. 双仓库并行（推荐）** | GitHub 保留为构建机与上游备份；Gitee 作为国内主仓库 / 镜像与下载源 | 构建链零改动；国内 clone 与下载走 Gitee；风险最低 | 两处仓库需同步（推送两次） | 想立刻解决下载慢，不想动 CI |
| **B. 全量迁移** | Gitee 成为唯一主仓库，CI 用 Gitee Go + 自托管 Windows runner，或本地构建后上传 Release | 彻底摆脱 GitHub | 需重建 CI、配置 secrets、维护 runner | 明确要完全脱离 GitHub |
| **C. 仅镜像下载** | 代码仍在 GitHub，仅把安装包与 `latest.json` 镜像到 Gitee | 改动最少 | 仍依赖 GitHub 访问仓库 | 只想加速下载 |

本指南按 **A → 按需叠加 B** 编写。若选 C，只做第 5、6 节与第 7 节的「下载相关」部分。

---

## 1. 迁移前检查与备份

### 1.1 当前状态（已核实）

```
remote origin  = git@github-qiqi:wolveszhu/qiqi.git   # SSH 别名，~/.ssh/config 映射到 github.com
分支           = main -> origin/main
tag            = v0.1.0 (08a7564)、v0.1.1 (52f4980，附注标签指向 ebe67b8)
版本           = 0.1.1（tauri.conf.json / Cargo.toml / package.json / latest.json 一致）
release/       = 4 个安装包已 tracked，共约 25 MB（.gitignore 忽略，当初用 git add -f 提交）
其它大文件     = qiqi-bootai-work-dist.zip（2.89 MB，含旧 latest.json）
```

### 1.2 备份

```powershell
cd d:\qiqi\qiqi
git bundle create ..\qiqi-backup-20261116.bundle --all
git status --porcelain        # 必须为空
```

检查 LFS / submodule：仓库当前**没有** `.gitattributes`、`.gitmodules`，无需 LFS 迁移。

### 1.3 大文件提醒

`release/` 下的安装包已进入历史 blob，`git rm --cached` 只删当前树、不减小克隆体积。若 Gitee 仓库空间吃紧，可选 `git filter-repo` 重写历史——但那会让所有旧 tag/commit hash 变化，**除非明确需要，不建议在迁移同时做**。

---

## 2. Gitee 仓库与凭据

### 2.1 创建仓库

1. 在 Gitee 新建公开仓库 `qiqi`，**不要**勾选初始化 README / .gitignore / License（避免与本地历史冲突）。
2. 若需要保留 Issues，用 Gitee 的「从 GitHub 导入仓库」功能做一次导入，再以本地 `git push` 覆盖代码与 tag；导入不保证 Release 资产与 Actions 配置完整，仍按后续步骤处理。

### 2.2 SSH 密钥

复用现有 `~/.ssh/qiqi_deploy.pub`，或生成新密钥：

```powershell
ssh-keygen -t ed25519 -C "qiqi@gitee" -f ~/.ssh/qiqi_gitee
Get-Content ~/.ssh/qiqi_gitee.pub | Set-Clipboard
```

把公钥粘贴到 Gitee → 设置 → SSH 公钥。

在 `~/.ssh/config` 追加（不要改动已有的 `github-qiqi` 别名，回滚要用）：

```
Host gitee-qiqi
    HostName gitee.com
    User git
    IdentityFile ~/.ssh/qiqi_gitee
    IdentitiesOnly yes
```

验证：`ssh -T git@gitee-qiqi`（首次连接需确认 host key）。

### 2.3 私人令牌（仅方案 B / 需要 API 上传 Release 时）

Gitee → 设置 → 私人令牌 → 生成新令牌，勾选仓库读写权限。令牌只写进 CI secrets 或本机环境变量，不要提交。

### 2.4 CI 凭据清单（方案 B）

GitHub Actions 当前注入 7 个构建期凭据，Gitee 侧需同名配置，否则内置额度/翻译能力会失效：

```
QIQI_ZHIPU_API_KEY
QIQI_SILICONFLOW_API_KEY
QIQI_BAIDU_APPID
QIQI_BAIDU_SECRET
QIQI_XFYUN_APPID
QIQI_XFYUN_API_SECRET
QIQI_XFYUN_API_KEY
```

它们经 `src-tauri/build.rs` 由 `option_env!` 在编译期注入，**不会**进前端产物；Gitee CI 中同样只能作为 secret 使用。

---

## 3. 代码与 tag 迁移

### 3.1 方案 A：双 remote（推荐）

```powershell
cd d:\qiqi\qiqi
git remote rename origin github                  # 原 GitHub 改名为备份
git remote add origin git@gitee-qiqi:<owner>/qiqi.git
git remote -v

git push origin main                             # 推代码
git push origin --tags                           # 推 v0.1.0 / v0.1.1
```

此后日常发布推两次：

```powershell
git push origin main; git push github main
git push origin v0.1.2; git push github v0.1.2   # 后者继续触发 GitHub Actions 构建
```

### 3.2 方案 B：完全切换

```powershell
git remote set-url origin git@gitee-qiqi:<owner>/qiqi.git
git remote add github-backup git@github-qiqi:wolveszhu/qiqi.git
git push origin main
git push origin --tags
```

确认无误前不要删除 `github-backup`。

### 3.3 验证

```powershell
git ls-remote origin refs/heads/main             # 应与本地 ebe67b8... 一致
git ls-remote --tags origin                      # 应含 v0.1.0 / v0.1.1
```

---

## 4. CI/CD（方案 B 才需要做）

### 4.1 现状

`.github/workflows/release.yml`：推 `v*` tag → `windows-latest` → pnpm/Rust 环境 → 静态发布数据守卫 → 7 个 secrets → `tauri-action@v0` 创建 Release（`releaseDraft: true`、`prerelease: false`）。

`website/.github/workflows/deploy.yml`：`website/**` 变更 → GitHub Pages 部署 VitePress 产物。

### 4.2 迁移选择

- **保留 GitHub Actions 继续构建**（方案 A）：什么都不用改，只需把构建出的安装包**同步上传**到 Gitee Release，并把 `latest.json` 的下载地址指向 Gitee。
- **改用 Gitee Go**：Gitee Go 的流水线配置格式、Windows runner 与 secrets 能力请以 Gitee 当前官方文档为准，本仓库不提供未经核验的 YAML。可复用的部分是本地脚本：`scripts/build-release.cmd -OutDir release` 会在本机完成 `verify:release-data` → `cargo test --test release_data_guard` → `pnpm tauri build -b msi,nsis` → 收集产物。自托管 Windows runner 直接调用它即可。
- **完全手工**：本机跑 `scripts/build-release.cmd`，到 Gitee Release 页面上传两个文件（见第 5 节）。

### 4.3 两个既有问题，迁移时顺手修掉

1. `.github/workflows/release.yml:94` 写的是 `qiqi_*_x64_en-US.nsis.exe`，实际产物是 `qiqi_<ver>_x64-setup.exe`，Release 说明里的文件名是错的。
2. `website/.github/workflows/deploy.yml:10` 的 paths 过滤写的是仓库根 `.github/workflows/deploy.yml`，而文件实际在 `website/.github/workflows/deploy.yml`。迁到任何平台前都要对齐触发路径。

### 4.4 构建工具链的 GitHub 依赖仍在

`scripts/build-release.ps1` 会从 `github.com/tauri-apps/nsis-tauri-utils/releases/...` 下载 NSIS 插件并校验 SHA1，Tauri CLI 自身还会下载 WiX/NSIS。因此**即使代码迁到 Gitee，打包机仍需能访问 GitHub release 或配置镜像**：

```powershell
.\scripts\build-release.cmd -GitHubMirror "https://ghfast.top/https://github.com"
```

已有镜像候选链见该脚本 `:158-169`。这一条是「完全脱离 GitHub」的最大实际障碍，需提前验证。

---

## 5. 发布与下载迁移

### 5.1 安装包命名（不要改）

```
MSI  : qiqi_<version>_x64_en-US.msi
NSIS : qiqi_<version>_x64-setup.exe
```

### 5.2 上传 Gitee Release

1. 进入 `https://gitee.com/<owner>/qiqi/releases`，新建 Release，标签选 `v0.1.1`。
2. 上传 `release/qiqi_0.1.1_x64_en-US.msi` 与 `release/qiqi_0.1.1_x64-setup.exe`。
3. 附件下载直链以 Gitee Release 页面实际生成的是准（不同版本路径规则可能调整），不要凭印象拼 URL。
4. 旧的 `v0.1.0` 资产建议保留，让已有用户仍能下载。

### 5.3 `latest.json`（仓库根，唯一真源）

```json
{
  "version": "0.1.1",
  "notes": "七七 v0.1.1：……",
  "downloadUrl": "https://gitee.com/<owner>/qiqi/releases"
}
```

改完后运行 `cd website && pnpm build`，`scripts/sync-latest.mjs` 会单向拷贝到 `website/docs/public/latest.json`。`website/.vitepress/site-test/latest.json` 是被忽略的旧测试残留（仍写 0.1.0），建议删除以免误用。

### 5.4 官网下载按钮

`website/.vitepress/theme/components/DownloadBox.vue`：默认 `downloadUrl` 与按钮文案「前往 GitHub Releases 下载」都要改；运行时文案来自 `latest.json.notes`，因此 notes 里也不要再提 GitHub。

---

## 6. 客户端「检查更新」源

`src-tauri/src/commands/update.rs` 的 `LATEST_JSON_URLS` 当前是三个 GitHub 源：raw + 两个 jsDelivr `gh/` 路径。改成 Gitee 可匿名读取的地址，例如：

```rust
const LATEST_JSON_URLS: &[&str] = &[
    "https://gitee.com/<owner>/qiqi/raw/main/latest.json",
    // 第二个源建议用自建静态托管（同一份 JSON），或 Gitee Pages 域名下的 /latest.json
];
```

注意：

- jsDelivr 的 `gh/` 路径只服务 GitHub，**不能直接换成 Gitee 路径**，如要用 CDN 请先核对 jsDelivr 当前对 Gitee 的支持。
- 该文件注释与单测里的示例 URL 也含 GitHub 字样，一并更新。
- `src/i18n/locales/{zh-CN,en-US}.ts` 的更新提示文案（「更新包托管于 GitHub Releases」）同步改为 Gitee。

---

## 7. 官网迁移

### 7.1 仓库链接与图标

`website/.vitepress/config.ts`：

- `const repo = "https://github.com/wolveszhu/qiqi"` → Gitee 地址
- `socialLinks: [{ icon: "github", link: repo }]` → `icon: "gitee"`（VitePress 若不支持该图标名，改为自定义链接或直接移除）

### 7.2 部署目标

当前是 GitHub Pages（`actions/deploy-pages@v4`），域名 `qiqi.bootai.work` 通过 CNAME 绑定。两条路：

- **Gitee Pages**：需按 Gitee 当前政策完成实名与自定义域名绑定，DNS 的 CNAME 目标要改；`website/docs/public/CNAME` 内容保持 `qiqi.bootai.work`（若平台不使用该文件，保留也无害）。
- **自建 Nginx（仓库已有配置）**：`website/nginx/qiqi.bootai.work.conf` 可直接用，把 `root /var/www/qiqi` 指向新服务器上的 `dist/` 内容；DNS 改为 A 记录指向该服务器，用 certbot 续期证书。这条路径不依赖任何代码托管平台，最稳。

### 7.3 页面文案

`website/docs/{download,privacy}.md` 与英文镜像里的「GitHub Releases」「项目地址」按第 8 节清单替换。

### 7.4 可选

官网字体走 `fonts.googleapis.com`、统计走 `hm.baidu.com`，与 GitHub 无关；若国内仍慢，可另行替换字体源，不影响迁移。

---

## 8. 文案与链接替换

逐文件、逐行的替换清单见 **[迁移文件清单.md](./迁移文件清单.md)**。执行原则：

1. 先改**功能性**文件（remote、`latest.json`、更新源、CI、脚本）；
2. 再改**用户可见**文案（官网、i18n、推广文章）；
3. 最后改**注释/设计文档**里的路径描述；
4. 历史提交记录、旧 tag、上游 issue 链接（如 `rust-lang/cargo`）不要改。

批量核对：

```powershell
cd d:\qiqi\qiqi
git grep -n -E 'github\.com/wolveszhu/qiqi|github-qiqi|releases/latest|GitHub Releases|gh repo set-default'
```

预期只剩清单中标注「可保留」的历史条目。

---

## 9. 迁移后验证清单

- [ ] `git remote -v` 与 `~/.ssh/config` 别名一致；`git ls-remote origin refs/heads/main` 等于本地 HEAD。
- [ ] 两个 tag 已推送且 hash 未变（`v0.1.0` → `08a7564`，`v0.1.1` → `52f4980`）。
- [ ] 匿名窗口能打开 Gitee 仓库与 Release，安装包可下载且 SHA256 与本地一致。
- [ ] `latest.json` 的 `downloadUrl` 从官网下载按钮点开后落在 Gitee Release。
- [ ] 客户端「检查更新」能读到新 `latest.json`，且网络受限时有第二源兜底。
- [ ] `https://qiqi.bootai.work/` 中英页面正常，`/download` 显示 0.1.1，社交图标不再指向 GitHub。
- [ ] DNS / CNAME 已切到新目标（若用 Gitee Pages 或自建 Nginx）。
- [ ] Release 说明里的安装包文件名与真实资产一致（修掉 `_nsis.exe` 旧写法）。
- [ ] 方案 B：推一个测试 tag，确认 Gitee CI 触发、7 个 secrets 生效、产物上传成功。
- [ ] 本地 `scripts/build-release.cmd` 仍能构建（含 GitHub 工具链镜像）。
- [ ] 全仓 grep 只剩余「可保留」清单中的 GitHub 引用。
- [ ] 旧 GitHub remote 与 tag 在确认稳定前保留，不删除。

---

## 10. 回滚

1. 切回远程：`git remote set-url origin git@github-qiqi:wolveszhu/qiqi.git`（或方案 A 下把 `origin` 换回 `github`）。
2. 恢复功能文件：`latest.json`、`update.rs`、`DownloadBox.vue`、i18n、官网文案——按第 8 节清单反向替换；最简单的是从 `v0.1.1` tag 检出这些文件。
3. 官网重新部署一次，DNS 改回原目标。
4. 备份包：`git bundle` 文件保留至少一个月，期间不要删。

---

## 11. 已知风险与 FAQ

**Q：Gitee 会不会因为 25 MB 安装包拒绝推送？**
A：Gitee 对仓库容量与单文件大小有限制，请以当前套餐规则为准。可以先只推代码与 tag，把 `release/` 从新仓库的历史中排除（新建干净仓库时可行；已导入历史则需 `git filter-repo`，代价见 §1.3）。

**Q：能不能只换下载链接，不迁仓库？**
A：可以（方案 C）。改 `latest.json`、`DownloadBox.vue`、i18n 与 `update.rs` 即可，官网与 CI 不动。

**Q：Gitee Go 能不能直接跑现有 release.yml？**
A：不能照搬。`tauri-apps/tauri-action@v0` 是 GitHub Actions 专属；secrets 语法、`github.ref_name` 上下文、`permissions` 都要重写。建议先在自托管 Windows 机器上跑通 `scripts/build-release.cmd`，再考虑接 CI。

**Q：迁移后 `v0.1.0`/`v0.1.1` tag 需要重打吗？**
A：不需要。`git push origin --tags` 会原样推送附注标签，hash 保持不变。

**Q：为什么不建议在迁移同时清理历史大文件？**
A：`filter-repo` 会改写所有 commit 与 tag，导致旧 tag 失效、用户手上的 clone 无法快进。先把迁移跑通，历史清理另立任务。
