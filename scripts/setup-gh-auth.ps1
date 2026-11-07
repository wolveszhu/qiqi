<#
.SYNOPSIS
  配置 GitHub CLI 认证（浏览器登录 / PAT 托管 / GITHUB_TOKEN 环境变量）。

.DESCRIPTION
  三种模式二选一（-Mode，默认 OAuth）：

    OAuth（推荐）  浏览器设备码授权。gh 自动托管凭据，本机不出现 token 明文，
                   不需要任何环境变量。适合日常手动 gh 命令、发 Release。

    Store          classic PAT 交给 gh auth login --with-token 加密托管。
                   适合不便开浏览器的场景，凭据不落环境变量。

    EnvVar         把 PAT 设为用户级 GH_TOKEN + GITHUB_TOKEN 环境变量。
                   gh 官方推荐 fine-grained PAT 走这种方式，也是第三方脚本 /
                   headless 自动化读取 GITHUB_TOKEN 的唯一方式。
                   注意：用户级环境变量以明文存在注册表，仅限本机个人账户使用。

  脚本对 PAT 一律用 Read-Host -AsSecureString 读取：不回显、不打印、不写入
  仓库文件，用完立即清除内存中的明文。绝不打印 token 内容。

  完成后验证 gh auth status，并设置默认仓库（本项目 remote 是 SSH 别名
  github-qiqi，gh 无法自动识别，必须显式指定，否则命令要加 -R owner/repo）。

.PARAMETER Mode
  OAuth | Store | EnvVar，默认 OAuth。

.PARAMETER Repo
  设为默认的仓库（owner/repo），默认 wolveszhu/qiqi。

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-gh-auth.ps1

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-gh-auth.ps1 -Mode EnvVar
#>
[CmdletBinding()]
param(
  [ValidateSet("OAuth", "Store", "EnvVar")]
  [string]$Mode = "OAuth",
  [string]$Repo = "wolveszhu/qiqi"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

# winget 安装后当前 shell 的 PATH 可能尚未刷新，这里合并机器级 + 用户级 PATH。
$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
            [Environment]::GetEnvironmentVariable("Path", "User")

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw "未找到 gh。请先运行：winget install --id GitHub.cli --exact --source winget"
}
Write-Host "gh 版本：" -NoNewline
gh --version | Select-Object -First 1

Write-Host ""
Write-Host "==> 1/3 GitHub 认证（模式：$Mode）" -ForegroundColor Cyan
Write-Host "    仓库：$Repo"

if ($Mode -eq "OAuth") {
  # 路径 A：浏览器设备码登录，gh 自己管理凭据，最安全。
  Write-Host "    即将打开浏览器授权流程（终端会显示一次性设备码）。" -ForegroundColor Yellow
  Write-Host "    若无法交互，可改用：-Mode Store 或 -Mode EnvVar" -ForegroundColor DarkGray
  $prevPref = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    gh auth login --hostname github.com --web --git-protocol https 2>&1 |
      ForEach-Object { "$_" }
    $code = $LASTEXITCODE
  }
  finally {
    $ErrorActionPreference = $prevPref
  }
  if ($code -ne 0) { throw "gh auth login 失败（退出码 $code）。" }
  Write-Host "    [OK] 浏览器授权完成，凭据由 gh 托管" -ForegroundColor Green
}
else {
  # Store / EnvVar 都需要 PAT：SecureString 保证不回显，明文只在内存中短暂停留。
  if ($Mode -eq "Store") {
    Write-Host "    请生成 classic PAT（tok_ 开头的旧格式不适用，应选 ghp_ 开头的 classic）。"
    Write-Host "    地址：https://github.com/settings/tokens/new   scopes 勾选 repo"
  }
  else {
    Write-Host "    推荐 fine-grained PAT，仅授权 wolveszhu/qiqi 的 Contents: Read and Write。"
    Write-Host "    地址：https://github.com/settings/personal-access-tokens/new"
  }
  $secure = Read-Host "    粘贴 PAT（输入不显示）" -AsSecureString
  $ptr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
  $plain = ""
  try {
    $plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr)
  }
  finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
  }
  if ([string]::IsNullOrWhiteSpace($plain)) {
    throw "未读取到 PAT，已中止。"
  }

  try {
    if ($Mode -eq "Store") {
      # 通过 stdin 交给 gh，避免 token 出现在命令行 / 进程列表 / 日志中。
      $prevPref = $ErrorActionPreference
      $ErrorActionPreference = "Continue"
      try {
        $plain | gh auth login --hostname github.com --with-token 2>&1 |
          ForEach-Object { "$_" }
        $code = $LASTEXITCODE
      }
      finally {
        $ErrorActionPreference = $prevPref
      }
      if ($code -ne 0) { throw "gh auth login 失败（退出码 $code）。" }
      Write-Host "    [OK] classic PAT 已交给 gh 加密托管（本机不留环境变量明文）" -ForegroundColor Green
    }
    else {
      # gh 官方：fine-grained PAT 走 GH_TOKEN；同时设 GITHUB_TOKEN 供第三方脚本读取。
      # 只设用户级（不设机器级），仅对当前 Windows 账户生效。
      [Environment]::SetEnvironmentVariable("GH_TOKEN", $plain, "User")
      [Environment]::SetEnvironmentVariable("GITHUB_TOKEN", $plain, "User")
      $env:GH_TOKEN = $plain
      $env:GITHUB_TOKEN = $plain
      Write-Host "    [OK] 已设置用户级 GH_TOKEN / GITHUB_TOKEN" -ForegroundColor Green
      Write-Host "    注意：环境变量以明文存于注册表；新开终端才会自动带上。" -ForegroundColor Yellow
      Write-Host "    撤销方式：到 GitHub 删该 PAT，再运行下面两行清空本机：" -ForegroundColor Yellow
      Write-Host '      [Environment]::SetEnvironmentVariable("GH_TOKEN", $null, "User")'
      Write-Host '      [Environment]::SetEnvironmentVariable("GITHUB_TOKEN", $null, "User")'
    }
  }
  finally {
    $plain = $null
    $secure = $null
    [GC]::Collect()
  }
}

Write-Host ""
Write-Host "==> 2/3 验证认证状态" -ForegroundColor Cyan
$prevPref = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
  gh auth status 2>&1 | ForEach-Object { "$_" }
  $authCode = $LASTEXITCODE
}
finally {
  $ErrorActionPreference = $prevPref
}
if ($authCode -ne 0) { throw "gh auth status 未通过，请重新登录。" }

Write-Host ""
Write-Host "==> 3/3 设置默认仓库（SSH 别名 remote 无法自动识别）" -ForegroundColor Cyan
$prevPref = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
  gh repo set-default $Repo 2>&1 | ForEach-Object { "$_" }
  $repoCode = $LASTEXITCODE
}
finally {
  $ErrorActionPreference = $prevPref
}
if ($repoCode -ne 0) {
  Write-Host "    警告：set-default 失败，可在使用时显式加 -R $Repo" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "==> 配置完成" -ForegroundColor Green
Write-Host "    常用命令："
Write-Host "      gh auth status"
Write-Host "      gh release list"
Write-Host "      gh release create v0.1.1 --title 'Qiqi v0.1.1' --notes-file release-notes.md .\release\*.msi .\release\*.exe"
Write-Host ""
Write-Host "    GITHUB_TOKEN 说明：" -ForegroundColor Yellow
Write-Host "    - 它不是能'安装'的软件，而是账号签发的凭据。" -ForegroundColor Yellow
Write-Host "    - 本机 gh 命令：OAuth/Store 模式已由 gh 托管，无需环境变量。" -ForegroundColor Yellow
Write-Host "    - GitHub Actions：CI 自带 secrets.GITHUB_TOKEN，无需手动配置。" -ForegroundColor Yellow
Write-Host "    - 只有第三方脚本 / headless 自动化才需要 EnvVar 模式的用户级变量。" -ForegroundColor Yellow
