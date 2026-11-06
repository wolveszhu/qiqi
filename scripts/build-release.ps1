<#
.SYNOPSIS
  七七桌面应用一键发布打包（本地产出 msi / nsis.exe，或触发 GitHub Actions 云端发布）。

.DESCRIPTION
  固化 .github/workflows/release.yml 的发布流程，方便本地一键复用：

    本地模式（默认）：
      前置检查 → pnpm install → pnpm build → 发布数据静态守卫
      → 干净库基线测试 → pnpm tauri build -b msi,nsis
      → 收集安装包到 release/ 并打印清单

    云端模式（-Publish）：
      前置检查 → 发布数据守卫 → 打 v<version> tag 并 push
      → 由 GitHub Actions 产出 Release（不消耗本地编译时间）

  发布数据守卫见 src-tauri/src/db/README.md §7.3；任一守卫失败即中止，
  绝不产出带测试数据的安装包。

.PARAMETER Publish
  改为触发 GitHub Actions 云端发布（打 tag + push），不在本地编译安装包。
  需工作区干净、且该版本 tag 不存在。

.PARAMETER SkipGuards
  跳过发布数据守卫。仅供本地反复调试构建速度时使用；发布时禁止使用。

.PARAMETER SkipInstall
  跳过 pnpm install（依赖已装好时省时）。

.PARAMETER DryRun
  只打印将要执行的步骤，不执行任何命令。用于核对流程 / 验证脚本。

.PARAMETER OutDir
  安装包收集目录（默认 <仓库根>/release，已被 .gitignore 忽略）。

.PARAMETER GitHubMirror
  下载 Windows 打包工具链时使用的 GitHub 镜像前缀。国内直连 GitHub release
  常超时（CLI 报 `failed to bundle project: timeout: global`）。也可直接设置
  环境变量 TAURI_BUNDLER_TOOLS_GITHUB_MIRROR。所有下载均校验官方 SHA1。

.EXAMPLE
  .\scripts\build-release.cmd
  双击运行：本地完整打包并输出 msi + nsis.exe。

.EXAMPLE
  .\scripts\build-release.ps1 -DryRun
  预览本地打包步骤。

.EXAMPLE
  .\scripts\build-release.ps1 -Publish
  打 tag 推送，触发 GitHub Actions 云端发布。
#>
[CmdletBinding()]
param(
  [switch]$Publish,
  [switch]$SkipGuards,
  [switch]$SkipInstall,
  [switch]$DryRun,
  [string]$OutDir,
  [string]$GitHubMirror
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0
$global:LASTEXITCODE = 0

# ---------- 定位仓库根（不写死绝对路径：规范 §K 冷眼三问③） ----------
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$TauriDir = Join-Path $RepoRoot "src-tauri"
$BundleDir = Join-Path $TauriDir "target\release\bundle"
if ([string]::IsNullOrWhiteSpace($OutDir)) {
  $OutDir = Join-Path $RepoRoot "release"
}

function Write-Step {
  param([string]$Message)
  Write-Host ""
  Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Ok {
  param([string]$Message)
  Write-Host "    [OK] $Message" -ForegroundColor Green
}

# 执行命令并校验退出码；DryRun 只打印不执行。
# Arguments 用数组 splatting，避免 "pnpm install --frozen-lockfile" 被当成单个参数。
function Invoke-Checked {
  param(
    [string]$Label,
    [string]$WorkDir,
    [string]$Command,
    [string[]]$Arguments = @()
  )
  $display = ($Arguments -join " ")
  if ([string]::IsNullOrWhiteSpace($display)) { $display = $Command } else { $display = "$Command $display" }
  Write-Host "    > $display"
  if ($DryRun) {
    Write-Host "    [DryRun] 跳过" -ForegroundColor DarkGray
    return
  }
  Push-Location $WorkDir
  $code = 1
  try {
    # pnpm/cargo 把进度信息写到 stderr。PS 5.1 在 `2>&1` 下会把每一行包装成
    # NativeCommandError（红色报错块），即使 $ErrorActionPreference=Continue 也一样，
    # 容易被误读为构建失败。用 ForEach-Object 把输出统一转成字符串再打印：
    # stderr 内容原样保留、不再产生 ErrorRecord，且 $LASTEXITCODE 仍准确。
    $prevPref = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
      & $Command @Arguments 2>&1 | ForEach-Object { "$_" }
      $code = $LASTEXITCODE
    }
    finally {
      $ErrorActionPreference = $prevPref
    }
  }
  finally {
    Pop-Location
  }
  if ($code -ne 0) {
    throw "$Label 失败（退出码 $code）。请查看上方输出，已中止发布。"
  }
  Write-Ok $Label
}

# ---------- Windows 打包工具链预检（规避 CLI 联网下载超时） ----------
# Tauri CLI 打包 NSIS 前会校验 %LOCALAPPDATA%\tauri\NSIS 缓存；缺
# Plugins\x86-unicode\additional\nsis_tauri_utils.dll 时会从 GitHub release 重下，
# 国内直连常超时并报 `failed to bundle project: timeout: global`。
# WiX 缓存通常独立完整，故典型现象是「MSI 成功、NSIS 卡住」。
# 这里提前补齐并按 CLI 内置的官方 SHA1 校验，使打包阶段无需联网。
$NSIS_PLUGIN_SHA1 = "75197FEE3C6A814FE035788D1C34EAD39349B860"
$NSIS_PLUGIN_PATH = "tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll"

function Ensure-NsisTools {
  $toolsDir = Join-Path $env:LOCALAPPDATA "tauri\NSIS"
  $dest = Join-Path $toolsDir "Plugins\x86-unicode\additional\nsis_tauri_utils.dll"

  if (Test-Path -LiteralPath $dest) {
    $hash = (Get-FileHash -LiteralPath $dest -Algorithm SHA1).Hash
    if ($hash -eq $NSIS_PLUGIN_SHA1) {
      Write-Ok "NSIS 工具链完整（插件 SHA1 校验通过）"
      return
    }
    Write-Host "    NSIS 插件哈希不符，需重新下载。" -ForegroundColor Yellow
  }
  else {
    Write-Host "    NSIS 插件缺失，需补齐（否则 CLI 联网重下易超时）。" -ForegroundColor Yellow
  }

  if ($DryRun) {
    Write-Host "    [DryRun] 跳过下载；真实运行会校验 $dest 的 SHA1" -ForegroundColor DarkGray
    return
  }

  # 候选源顺序：用户指定镜像 → CLI 已有镜像环境变量 → GitHub 直连 → 常用公共镜像。
  $sources = @()
  if (-not [string]::IsNullOrWhiteSpace($GitHubMirror)) {
    $sources += $GitHubMirror.TrimEnd("/")
  }
  if (-not [string]::IsNullOrWhiteSpace($env:TAURI_BUNDLER_TOOLS_GITHUB_MIRROR)) {
    $sources += $env:TAURI_BUNDLER_TOOLS_GITHUB_MIRROR.TrimEnd("/")
  }
  $sources += "https://github.com"
  $sources += "https://ghfast.top/https://github.com"
  $sources += "https://gh-proxy.com/https://github.com"
  $sources = @($sources | Select-Object -Unique)

  $tmp = Join-Path $env:TEMP ("nsis_tauri_utils_" + [guid]::NewGuid().ToString("N") + ".dll")
  $downloaded = $false
  $usedMirror = $null
  try {
    foreach ($src in $sources) {
      $url = "$src/$NSIS_PLUGIN_PATH"
      try {
        Write-Host "    尝试下载 $url"
        Invoke-WebRequest -Uri $url -OutFile $tmp -TimeoutSec 20 -UseBasicParsing -ErrorAction Stop
        $hash = (Get-FileHash -LiteralPath $tmp -Algorithm SHA1).Hash
        if ($hash -ne $NSIS_PLUGIN_SHA1) {
          throw "SHA1 不匹配（$hash），拒绝使用"
        }
        $downloaded = $true
        if ($src -ne "https://github.com") { $usedMirror = $src }
        break
      }
      catch {
        Write-Host "    失败：$($_.Exception.Message)" -ForegroundColor DarkGray
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
      }
    }
  }
  finally {
    if ($downloaded) {
      New-Item -ItemType Directory -Path (Split-Path -Path $dest -Parent) -Force | Out-Null
      Copy-Item -LiteralPath $tmp -Destination $dest -Force
      Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
    }
  }

  if (-not $downloaded) {
    $hint = @(
      "NSIS 工具链插件下载失败，无法离线打包。",
      "原因：CLI 校验到 %LOCALAPPDATA%\tauri\NSIS 缺 nsis_tauri_utils.dll，重下时 GitHub 超时。",
      "处理：联网后重试，或指定可访问的 GitHub 镜像：",
      '  .\scripts\build-release.cmd -GitHubMirror "https://ghfast.top/https://github.com"'
    ) -join [Environment]::NewLine
    throw $hint
  }
  Write-Ok "已补齐 NSIS 插件（SHA1 校验通过）"

  # CLI 若还需下载其他工具（WiX / NSIS 主体），一并走镜像，避免再次超时。
  if ($usedMirror -and [string]::IsNullOrWhiteSpace($env:TAURI_BUNDLER_TOOLS_GITHUB_MIRROR)) {
    $env:TAURI_BUNDLER_TOOLS_GITHUB_MIRROR = $usedMirror
    Write-Host "    已设置 TAURI_BUNDLER_TOOLS_GITHUB_MIRROR=$usedMirror" -ForegroundColor DarkGray
  }
}

# ---------- 1. 前置检查 ----------
Write-Step "1/6 前置检查"

$confPath = Join-Path $TauriDir "tauri.conf.json"
if (-not (Test-Path $confPath)) {
  throw "找不到 $confPath，请在仓库内运行本脚本。"
}
$conf = Get-Content -LiteralPath $confPath -Raw | ConvertFrom-Json
$Version = $conf.version
$ProductName = $conf.productName
if ([string]::IsNullOrWhiteSpace($Version)) {
  throw "tauri.conf.json 缺少 version。"
}
Write-Ok "产品 $ProductName v$Version"

foreach ($tool in @("node", "pnpm", "cargo", "git")) {
  if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
    throw "缺少 $tool，请先安装并加入 PATH。"
  }
}
Write-Ok "node / pnpm / cargo / git 可用"

$targets = $conf.bundle.targets
if ($targets -is [array]) { $targets = $targets -join "," }
if ($targets -notmatch "msi" -or $targets -notmatch "nsis") {
  throw "tauri.conf.json 的 bundle.targets 需同时含 msi 与 nsis，当前为: $targets"
}
Write-Ok "打包目标 $targets"

# 本地首次构建 Tauri 会自动下载 WiX / NSIS 工具链，需联网；此处仅提示不阻断。
if (-not $Publish) {
  $wix = Get-Command light -ErrorAction SilentlyContinue
  $nsis = Get-Command makensis -ErrorAction SilentlyContinue
  if (-not $wix -or -not $nsis) {
    Write-Host "    提示：PATH 中未找到 WiX / NSIS，Tauri 首次构建会自动下载工具链，请保持联网。" -ForegroundColor Yellow
  }
}

# ---------- 2. 云端模式：校验并打 tag（提前于本地编译，避免白跑） ----------
$tag = "v$Version"
if ($Publish) {
  $prevPref = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    $status = @(& git -C $RepoRoot status --porcelain)
  }
  finally {
    $ErrorActionPreference = $prevPref
  }
  if ($status.Count -gt 0) {
    $details = $status -join [Environment]::NewLine
    if ($DryRun) {
      Write-Host "    [DryRun] 工作区未提交（真实发布时会在此中止）：" -ForegroundColor Yellow
      Write-Host "    $details" -ForegroundColor DarkGray
    }
    else {
      throw "工作区有未提交改动，禁止发布。请先提交：$([Environment]::NewLine)$details"
    }
  }
  else {
    Write-Ok "工作区干净"
  }

  $prevPref = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    & git -C $RepoRoot rev-parse -q --verify "refs/tags/$tag" 2>&1 | Out-Null
    $tagExists = ($LASTEXITCODE -eq 0)
  }
  finally {
    $ErrorActionPreference = $prevPref
  }
  if ($tagExists) {
    if ($DryRun) {
      Write-Host "    [DryRun] tag $tag 已存在（真实发布时会在此中止，需提升 version）" -ForegroundColor Yellow
    }
    else {
      throw "tag $tag 已存在。发布新版本请先在 tauri.conf.json 提升 version。"
    }
  }
  else {
    Write-Ok "tag $tag 不存在"
  }

  # 云端构建同样要先过数据守卫，避免把脏数据推上去。
  if ($SkipGuards) {
    Write-Host "    警告：-Publish 下忽略 -SkipGuards，发布前守卫强制执行。" -ForegroundColor Yellow
    $SkipGuards = $false
  }
}

# ---------- 3. 前端依赖与构建 ----------
if (-not $Publish) {
  Write-Step "2/6 前端依赖与构建"
  if ($SkipInstall) {
    Write-Host "    跳过 pnpm install（-SkipInstall）" -ForegroundColor Yellow
  }
  else {
    Invoke-Checked -Label "安装前端依赖" -WorkDir $RepoRoot -Command "pnpm" -Arguments @("install", "--frozen-lockfile")
  }
  # 显式先构建前端，让静态守卫能扫描即将嵌入二进制的 dist/。
  Invoke-Checked -Label "构建前端" -WorkDir $RepoRoot -Command "pnpm" -Arguments @("build")
}

# ---------- 4. 发布数据守卫 ----------
Write-Step "3/6 发布数据守卫"
if ($SkipGuards) {
  Write-Host "    跳过守卫（-SkipGuards）；此构建不可用于发布！" -ForegroundColor Yellow
}
else {
  Invoke-Checked -Label "静态守卫（构建输入）" -WorkDir $RepoRoot -Command "pnpm" -Arguments @("verify:release-data")
  Invoke-Checked -Label "运行时守卫（干净库基线）" -WorkDir $TauriDir -Command "cargo" -Arguments @("test", "--test", "release_data_guard")
}

# ---------- 5a. 云端发布 ----------
if ($Publish) {
  Write-Step "4/6 推送 tag 触发 GitHub Actions"
  Invoke-Checked -Label "推送 main" -WorkDir $RepoRoot -Command "git" -Arguments @("push", "origin", "main")
  Invoke-Checked -Label "创建 tag" -WorkDir $RepoRoot -Command "git" -Arguments @("tag", $tag)
  Invoke-Checked -Label "推送 tag" -WorkDir $RepoRoot -Command "git" -Arguments @("push", "origin", $tag)
  Write-Step "5/6 已触发云端构建"
  Write-Host "    Actions:  https://github.com/wolveszhu/qiqi/actions" -ForegroundColor Cyan
  Write-Host "    Releases: https://github.com/wolveszhu/qiqi/releases" -ForegroundColor Cyan
  Write-Host "    完成后 Release 为草稿，需到网页 Publish。" -ForegroundColor Yellow
  Write-Step "6/6 完成（云端）"
  exit 0
}

# ---------- 5b. 本地构建安装包 ----------
Write-Step "4/6 编译并打包 Tauri（首次约 10–20 分钟）"
Ensure-NsisTools
try {
  Invoke-Checked -Label "tauri build" -WorkDir $RepoRoot -Command "pnpm" -Arguments @("tauri", "build", "-b", "msi,nsis")
}
catch {
  # CLI 的 `timeout: global` 只说超时、不说原因，这里补上可行动的诊断。
  Write-Host "    若日志含 'failed to bundle project: timeout: global'：" -ForegroundColor Yellow
  Write-Host "    Tauri 仍在联网下载 WiX/NSIS 工具链（GitHub release 直连超时），" -ForegroundColor Yellow
  Write-Host '    可指定镜像重跑： .\scripts\build-release.cmd -GitHubMirror "https://ghfast.top/https://github.com"' -ForegroundColor Yellow
  throw
}

# ---------- 6. 收集产物 ----------
Write-Step "5/6 收集安装包"
if ($DryRun) {
  Write-Host "    [DryRun] 将扫描 $BundleDir 并把 *.msi / *.exe 复制到 $OutDir" -ForegroundColor DarkGray
  Write-Step "6/6 完成（DryRun）"
  exit 0
}

if (-not (Test-Path $BundleDir)) {
  throw "未找到产物目录 $BundleDir。tauri build 可能未生成 bundle，请查看上方日志。"
}
# 只从 Tauri 的标准输出目录取安装包，避免误收 WiX / NSIS 自带的工具 exe。
$artifacts = @()
foreach ($sub in @("msi", "nsis")) {
  $subDir = Join-Path $BundleDir $sub
  if (Test-Path $subDir) {
    $artifacts += @(Get-ChildItem -LiteralPath $subDir -File -ErrorAction SilentlyContinue |
      Where-Object { $_.Extension -in @(".msi", ".exe") })
  }
}
if ($artifacts.Count -eq 0) {
  throw "$BundleDir 下没有 msi / exe 安装包。"
}

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$collected = @()
foreach ($item in $artifacts) {
  $dest = Join-Path $OutDir $item.Name
  Copy-Item -LiteralPath $item.FullName -Destination $dest -Force
  $collected += $dest
}

Write-Step "6/6 完成"
foreach ($file in $collected) {
  $sizeMB = [math]::Round((Get-Item -LiteralPath $file).Length / 1MB, 2)
  Write-Host ("    {0}  ({1} MB)" -f $file, $sizeMB) -ForegroundColor Green
}
Write-Host ""
Write-Host "    提示：安装包未签名，Windows SmartScreen 可能拦截，选「仍要运行」。" -ForegroundColor Yellow
Write-Host ("    验收全新安装前，请先清空 %APPDATA%\{0}" -f $conf.identifier) -ForegroundColor Yellow
