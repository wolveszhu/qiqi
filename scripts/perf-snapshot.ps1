<#
.SYNOPSIS
  七七桌宠进程树性能快照（E2 测量基建，perf-snapshot）。

.DESCRIPTION
  口径见《性能-E2性能与隐私达标实施计划》§一#1：
    - 内存 = 进程树**私有工作集总和**（主进程 qiqi.exe + 全部 WebView2 子进程）；
    - CPU  = 两次采样的累计 CPU 时间差 / 墙钟差，输出
        · CpuMachinePct：全机基准（Task Manager 风格，100% = 所有核心）——红线判定用；
        · CpuCorePct    ：单核占比（0..N*100%）——参考。
  以**主进程 PID 为根递归聚合进程树**（非按进程名匹配），避免误计同名无关进程（冷眼三问②）。

.PARAMETER ProcessName
  主进程名（不含 .exe，默认 qiqi）。
.PARAMETER IntervalSec
  采样间隔秒（默认 5）。
.PARAMETER Samples
  采样次数（默认 120 ≈ 10 分钟；首个样本仅建基线不计入）。
.PARAMETER OutCsv
  输出 CSV 路径（默认 scripts/perf-<时间戳>.csv）；环境信息另写同名 .env.txt。

.EXAMPLE
  .\scripts\perf-snapshot.ps1 -Samples 120 -IntervalSec 5
.EXAMPLE
  # 仅吸附休眠场景：拖到边缘后再启动
  .\scripts\perf-snapshot.ps1 -Samples 120 -IntervalSec 5 -OutCsv scripts\perf-sleep.csv
#>
[CmdletBinding()]
param(
  [string]$ProcessName = "qiqi",
  [double]$IntervalSec = 5,
  [int]$Samples = 120,
  [string]$OutCsv
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

# ---------- 环境登记（§一#5：数据可复现） ----------
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$logical = [int]$cpu.NumberOfLogicalProcessors
$envLines = @()
$envLines += ("snapshot_time : {0}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"))
$envLines += ("os            : {0} (build {1})" -f $os.Caption.Trim(), $os.Version)
$envLines += ("cpu           : {0} | {1}C/{2}T" -f $cpu.Name.Trim(), $cpu.NumberOfCores, $logical)
$envLines += ("ram_total     : {0:0.0} GB" -f ($os.TotalVisibleMemorySize / 1MB))
try {
  $pp = (& powercfg /getactivescheme) 2>$null
  $envLines += ("power_scheme  : {0}" -f (($pp -replace '.*GUID:\s*','') -replace '\s*\(.*\)','').Trim())
} catch { $envLines += "power_scheme  : (读取失败，请在'电源选项'人工确认)" }
$envLines += "refresh_rate  : 请在「设置 > 系统 > 显示 > 高级显示」人工确认（影响 vsync 功耗判定）"
$envLines += ("red_line      : 待机 CPU < 2%（全机基准）且 进程树 MEM < 150 MB")

Write-Host "==== 测量环境 ===="
$envLines | ForEach-Object { Write-Host $_ }

# ---------- 定位主进程（取最早启动的实例为根；桌宠单例） ----------
function Get-RootPid([string]$name) {
  $procs = @(Get-Process -Name $name -ErrorAction SilentlyContinue)
  if ($procs.Count -eq 0) { return $null }
  return ($procs | Sort-Object StartTime | Select-Object -First 1).Id
}

# ---------- 以 root 为祖先递归聚合进程树（父链） ----------
function Get-TreePids([int]$root) {
  $all = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
  $set = New-Object 'System.Collections.Generic.HashSet[int]'
  [void]$set.Add($root)
  $changed = $true
  while ($changed) {
    $changed = $false
    foreach ($p in $all) {
      $pid = [int]$p.ProcessId
      $ppid = [int]$p.ParentProcessId
      if ($set.Contains($ppid) -and -not $set.Contains($pid)) {
        [void]$set.Add($pid); $changed = $true
      }
    }
  }
  return $set
}

function Get-TreeSample($pids) {
  $cpu = 0.0
  $mem = 0L
  $alive = 0
  $names = @{}
  foreach ($proc in @(Get-Process -ErrorAction SilentlyContinue)) {
    if ($pids.Contains($proc.Id)) {
      $alive++
      $cpu += [double]$proc.CPU
      $mem += [int64]$proc.PrivateWorkingSet64
      $names[$proc.ProcessName] = 1
    }
  }
  return [pscustomobject]@{ Cpu = $cpu; Mem = $mem; Alive = $alive; Names = ($names.Keys -join ",") }
}

$root = Get-RootPid $ProcessName
if ($null -eq $root) {
  Write-Error ("未找到进程 '{0}.exe'——请先启动桌宠再测量。" -f $ProcessName)
  exit 2
}
Write-Host "==== 目标进程树 ===="
Write-Host ("root PID = {0}" -f $root)

# ---------- 输出路径 ----------
if ([string]::IsNullOrWhiteSpace($OutCsv)) {
  $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
  $dir = if ($PSScriptRoot) { $PSScriptRoot } else { "scripts" }
  $OutCsv = Join-Path $dir ("perf-{0}.csv" -f $stamp)
}
$envPath = "$OutCsv.env.txt"
$envLines | Set-Content -Path $envPath -Encoding UTF8

$rows = New-Object 'System.Collections.Generic.List[object]'
$prev = $null
$prevTime = $null
Write-Host "==== 采样开始 ===="
Write-Host ("{0} 次 × {1}s（首样本建基线不计） → {2}" -f $Samples, $IntervalSec, $OutCsv)

for ($i = 0; $i -lt $Samples; $i++) {
  $now = Get-Date
  $pids = Get-TreePids $root
  $s = Get-TreeSample $pids

  if ($null -ne $prev) {
    $dt = ($now - $prevTime).TotalSeconds
    if ($dt -le 0) { $dt = $IntervalSec }
    $cores = ($s.Cpu - $prev.Cpu) / $dt            # 平均占用核心数
    $row = [pscustomobject]@{
      Time          = $now.ToString("HH:mm:ss")
      CpuMachinePct = [math]::Round(($cores / $logical) * 100, 2)   # 全机基准（红线）
      CpuCorePct    = [math]::Round($cores * 100, 2)                # 单核占比（参考）
      MemMB         = [math]::Round($s.Mem / 1MB, 1)
      Procs         = $s.Alive
      Names         = $s.Names
    }
    $rows.Add($row)
    Write-Host ("{0}  CPU {1:N2}% (单核 {2:N2}%)  MEM {3:N1} MB  procs {4}  [{5}]" -f `
        $row.Time, $row.CpuMachinePct, $row.CpuCorePct, $row.MemMB, $row.Procs, $row.Names)
  } else {
    Write-Host ("基线: procs {0} [{1}]  MEM {2:N1} MB" -f $s.Alive, $s.Names, ($s.Mem / 1MB))
  }
  $prev = $s
  $prevTime = $now
  if ($i -lt $Samples - 1) { Start-Sleep -Seconds $IntervalSec }
}

# ---------- 汇总 ----------
if ($rows.Count -gt 0) {
  $cpuAvg = ($rows | Measure-Object CpuMachinePct -Average).Average
  $cpuMax = ($rows | Measure-Object CpuMachinePct -Maximum).Maximum
  $memAvg = ($rows | Measure-Object MemMB -Average).Average
  $memMax = ($rows | Measure-Object MemMB -Maximum).Maximum
  $rows | Export-Csv -Path $OutCsv -NoTypeInformation -Encoding UTF8
  Write-Host "==== 汇总 ===="
  Write-Host ("CPU(全机)  均值 {0:N2}%  峰值 {1:N2}%" -f $cpuAvg, $cpuMax)
  Write-Host ("进程树 MEM 均值 {0:N1} MB  峰值 {1:N1} MB" -f $memAvg, $memMax)
  $pass = ($cpuAvg -lt 2.0) -and ($memAvg -lt 150.0)
  Write-Host ("红线判定（均值）：{0}" -f $(if ($pass) { "PASS" } else { "FAIL（待机应 CPU<2% 且 MEM<150MB）" }))
  Write-Host ("CSV  → {0}" -f $OutCsv)
  Write-Host ("ENV  → {0}" -f $envPath)
} else {
  Write-Warning "有效样本不足（Samples<2），未生成 CSV。"
}
