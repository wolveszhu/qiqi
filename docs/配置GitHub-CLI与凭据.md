# 配置 GitHub CLI 与凭据（本机）

> 最近核对：2026-11-07。适用仓库 `wolveszhu/qiqi`（public），本机 Windows x64。
> 相关脚本：`scripts/setup-gh-auth.ps1`（UTF-8 BOM，PowerShell 5.1 可运行）。

---

## 0. 先厘清一个概念

| 名称 | 是什么 | 怎么"装" |
| :--- | :--- | :--- |
| `gh` CLI | 一个软件 | `winget install --id GitHub.cli --exact --source winget`（**本机已装好，v2.103.3**） |
| `GITHUB_TOKEN` / `GH_TOKEN` | **不是软件**，是你 GitHub 账号签发的凭据字符串（环境变量形式） | 不能由他人代生成；需你本人在 GitHub 网页签发 PAT 或走浏览器 OAuth |
| `secrets.GITHUB_TOKEN` | GitHub Actions 每次运行时**自动注入**的临时令牌 | 无需任何手动配置，`release.yml` 已在用 |

**日常手动用 `gh` 命令（发 Release、查资产）根本不需要环境变量**——`gh auth login` 后凭据由 gh 托管。只有第三方脚本 / headless 自动化明确读取 `GITHUB_TOKEN` 时，才需要把它设成环境变量。

本机当前状态：

- `gh` 2.103.3 已通过 winget 用户级安装，用户级 PATH 已持久化，新开终端可直接用。
- `gh auth status` 显示未登录（认证必须本人完成）。
- 无 `GH_TOKEN` / `GITHUB_TOKEN` 环境变量，无 `hosts.yml`。
- 仓库 remote 是 SSH 别名 `git@github-qiqi:wolveszhu/qiqi.git`（`~/.ssh/config` 映射到 `github.com`，密钥 `qiqi_deploy`）。**gh 无法从别名识别仓库**，所以脚本会执行 `gh repo set-default`；否则所有 `gh` 命令都要显式加 `-R wolveszhu/qiqi`。

---

## 1. 三种认证方式怎么选

| 模式 | 适用场景 | token 是否出现明文 | 安全性 |
| :--- | :--- | :--- | :--- |
| **OAuth（默认，推荐）** | 日常手动 `gh` 命令、发 Release | 否，浏览器授权后 gh 托管 | 最高 |
| **Store** | 不便开浏览器；用 classic PAT | 输入时不回显，之后由 gh 加密托管 | 高 |
| **EnvVar** | fine-grained PAT、第三方脚本、headless 自动化 | 是，用户级环境变量以明文存注册表 | 中（仅限本机个人账户） |

> gh 官方建议：**fine-grained PAT 用 `GH_TOKEN` 环境变量**（即 EnvVar 模式），因为它的资源范围受限，用 `--with-token` 交给 gh 托管后，访问其他资源时容易出现令人困惑的行为。classic PAT 才适合 Store 模式。

---

## 2. 方式 A：浏览器 OAuth（推荐，一条命令）

**新开一个 PowerShell 窗口**（winget 安装后需刷新 PATH），执行：

```powershell
cd d:\qiqi\qiqi
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-gh-auth.ps1
```

或直接用 gh 原生命令：

```powershell
gh auth login --hostname github.com --web --git-protocol https
```

流程：

1. 终端显示 `! First copy your one-time code: XXXX-XXXX`，复制这串码。
2. 按提示打开 <https://github.com/login/device>（脚本模式会尝试自动打开浏览器）。
3. 粘贴一次性码 → `Continue` → 授权 `GitHub CLI`。
4. 回到终端，看到 `Authentication complete` 即完成。

脚本随后自动验证 `gh auth status` 并设置默认仓库。

---

## 3. 方式 B：classic PAT 交给 gh 托管

### 3.1 生成 PAT

1. 打开 <https://github.com/settings/tokens/new>（可能需再次输入密码）。
2. `Note` 填 `qiqi-local-gh`；`Expiration` 建议 90 天（不要选 `No expiration`）。
3. `Select scopes` 勾 **`repo`**（发 Release 需要 `Contents: write`；classic 的 `repo` 已包含）。
4. 点 `Generate token`，**立刻复制** `ghp_...` 开头的串（离开页面后不再显示）。

### 3.2 录入

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-gh-auth.ps1 -Mode Store
```

提示「粘贴 PAT」时右键粘贴（输入不回显）。脚本通过 stdin 把 token 交给
`gh auth login --with-token`，**不出现在命令行、进程列表或日志里**，用完清除内存中的明文。

---

## 4. 方式 C：设置 `GITHUB_TOKEN` 环境变量（你提到的那种）

### 4.1 生成 fine-grained PAT（推荐，权限最小）

1. 打开 <https://github.com/settings/personal-access-tokens/new>。
2. `Token name` 填 `qiqi-local-scripts`；`Expiration` 建议 90 天。
3. `Resource owner` 选 `wolveszhu`；`Repository access` 选 **Only select repositories** → `qiqi`。
4. `Permissions` → `Repository permissions` → `Contents` 设为 **Read and Write**（发 Release 必需）；
   如需读 Actions 另加 `Actions: Read`。`Metadata: Read` 会自动带上。
5. 点 `Generate token` 并复制 `github_pat_...` 开头的串。

> 也可用 classic PAT（勾 `repo`），但 fine-grained 可把权限锁死在单个仓库，泄露影响面更小。

### 4.2 录入（脚本方式，推荐）

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-gh-auth.ps1 -Mode EnvVar
```

脚本会设置**用户级** `GH_TOKEN` 和 `GITHUB_TOKEN`（只影响当前 Windows 账户，不改机器级），
并在当前进程也生效以便立即验证。

### 4.3 手动方式（等价，注意会显示明文）

```powershell
# 在当前窗口临时生效（关闭即失效，适合临时测试）
$env:GH_TOKEN = "<粘贴你的 PAT>"
$env:GITHUB_TOKEN = $env:GH_TOKEN

# 永久写入用户级环境变量（重启终端后生效）
[Environment]::SetEnvironmentVariable("GH_TOKEN", "<粘贴你的 PAT>", "User")
[Environment]::SetEnvironmentVariable("GITHUB_TOKEN", "<粘贴你的 PAT>", "User")
```

> 手动方式会把 token 明文留在 PowerShell 历史/屏幕录制里，能避免就避免。脚本用 `Read-Host -AsSecureString`，更安全。

---

## 5. 验证

```powershell
gh --version
gh auth status                       # 应显示 Logged in to github.com
gh repo view --json nameWithOwner     # 应返回 wolveszhu/qiqi
gh release list                      # 目前为空（远端还没有 Release）
```

环境变量模式额外验证（**不要打印值**）：

```powershell
echo "GH_TOKEN 已设置: $([bool]$env:GH_TOKEN)"
echo "GITHUB_TOKEN 已设置: $([bool]$env:GITHUB_TOKEN)"
```

> 环境变量需**新开终端**才会自动带上；winget 安装 gh 后也一样。

---

## 6. 配置好后：把安装包发成 Release

```powershell
cd d:\qiqi\qiqi
gh release create v0.1.0 `
  --title "Qiqi v0.1.0" `
  --notes-file release-notes.md `
  .\release\qiqi_0.1.0_x64_en-US.msi `
  .\release\qiqi_0.1.0_x64-setup.exe
```

`v0.1.0` tag 已存在，若 gh 提示 tag 问题，加 `--verify-tag`；若想基于最新代码发布，
先提升 `src-tauri/tauri.conf.json` 的 `version` 再打新 tag。完整发布教程见
[`docs/发布安装包到GitHub.md`](发布安装包到GitHub.md) §3 / §4。

**重要**：`release.yml` 里 `releaseDraft: true` 只影响 CI 自动发布；用 `gh release create`
手动创建默认是**已发布**，无需再点 Publish（除非你加了 `--draft`）。

---

## 7. 安全红线

1. **绝不把 PAT 提交到仓库**：`.gitignore` 已忽略 `.env` / `.env.*`，但仍不要写进任何被跟踪的文件。
2. **绝不把 PAT 贴给聊天窗口 / 工单 / 日志**；脚本不会打印 token。
3. **不要设机器级环境变量**：用 `User` 级即可，避免同机其他账户读到。
4. **设过期时间**：建议 90 天，到期前轮换。
5. **权限最小化**：fine-grained PAT 只授权 `qiqi` 仓库的 `Contents: Read and Write`。
6. **PAT 与 OAuth 凭据不要混用**：`GH_TOKEN` 优先级高于 gh 托管的登录凭据，若两者并存且 PAT 权限更小，gh 命令可能"莫名"失败。排查时先 `echo $([bool]$env:GH_TOKEN)` 看是否环境变量在生效。

---

## 8. 撤销 / 轮换

**在 GitHub 侧撤销（首要）**：<https://github.com/settings/tokens> → 删除对应 PAT；
或在 `gh auth status` 显示的 OAuth 授权里，到 <https://github.com/settings/applications> 撤销 `GitHub CLI`。

**清空本机环境变量**：

```powershell
[Environment]::SetEnvironmentVariable("GH_TOKEN", $null, "User")
[Environment]::SetEnvironmentVariable("GITHUB_TOKEN", $null, "User")
Remove-Item Env:\GH_TOKEN -ErrorAction SilentlyContinue
Remove-Item Env:\GITHUB_TOKEN -ErrorAction SilentlyContinue
```

**退出 gh 登录**：

```powershell
gh auth logout --hostname github.com
```

---

## 9. 常见问题

| 现象 | 原因 / 处理 |
| :--- | :--- |
| `gh: command not found` | winget 装完当前 shell 未刷新 PATH，**新开终端**即可；或脚本里已自动合并机器级+用户级 PATH |
| `To get started with GitHub CLI, please run: gh auth login` | 未认证，按 §2 / §3 / §4 配置 |
| `gh repo view` 报识别不了仓库 | remote 是 SSH 别名 `github-qiqi`；脚本已 `gh repo set-default`，或命令加 `-R wolveszhu/qiqi` |
| `gh release create` 报权限不足 | PAT 缺 `Contents: Write`（fine-grained）或 `repo` scope（classic）；重新签发 |
| fine-grained PAT 用 `--with-token` 后行为异常 | 官方建议改用 `GH_TOKEN`（EnvVar 模式），见 §1 |
| `git push` 仍失败 | gh 认证与 git remote 认证是两套：本项目 remote 用 SSH deploy key `~/.ssh/qiqi_deploy`，与 gh token 无关 |
| `release.yml` 里的 `GITHUB_TOKEN` 要配吗 | 不用，GitHub Actions 自动注入 `secrets.GITHUB_TOKEN` |
