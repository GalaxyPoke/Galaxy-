# 生成 Mac 打包材料（在 Windows 上运行）：dist-mac-kit\GalaxyPokemon-mac-kit\ 和同名 zip
#   source\                ← 本仓库源码（不含 node_modules、target 等编译产物）
#   mac-client\.minecraft\ ← 整合包，去掉了 Windows 的 Java（runtime\）和本地库（*-natives）
#   先看我.txt
# 拷到 Mac 上解压，在 source 目录里运行：bash galaxy-build-mac.sh ../mac-client/.minecraft
# 用法：powershell -ExecutionPolicy Bypass -File galaxy-pack-mac-kit.ps1 [-Client <.minecraft 目录>] [-NoZip]
param(
  [string]$Client = 'D:\GalaxyPokemon\GalaxyPokemon-Launcher\client\.minecraft',
  [switch]$NoZip
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$versionId = '1.21.1-Fabric 0.19.3'
if (-not (Test-Path -LiteralPath (Join-Path $Client "versions\$versionId\$versionId.json"))) {
  throw "整合包目录不对（缺少版本 $versionId）：$Client"
}

$outDir = Join-Path $root 'dist-mac-kit'
$kit = Join-Path $outDir 'GalaxyPokemon-mac-kit'
if (Test-Path -LiteralPath $kit) { Remove-Item -LiteralPath $kit -Recurse -Force }
New-Item -ItemType Directory -Force $kit | Out-Null

function Copy-Tree([string]$From, [string]$To, [string[]]$ExcludeDirs = @(), [string[]]$ExcludeFiles = @()) {
  # robocopy 退出码 < 8 都算成功
  $rc = @($From, $To, '/E', '/NFL', '/NDL', '/NJH', '/NJS', '/NP', '/R:1', '/W:1')
  if ($ExcludeDirs.Count) { $rc += '/XD'; $rc += $ExcludeDirs }
  if ($ExcludeFiles.Count) { $rc += '/XF'; $rc += $ExcludeFiles }
  & robocopy @rc | Out-Null
  if ($LASTEXITCODE -ge 8) { throw "复制失败：$From -> $To（robocopy $LASTEXITCODE）" }
}

Write-Output '复制源码…'
Copy-Tree $root (Join-Path $kit 'source') `
  -ExcludeDirs @('node_modules', '.turbo', (Join-Path $root 'target'), (Join-Path $root 'dist-launcher'), (Join-Path $root 'dist-mac'), $outDir, (Join-Path $root '.git')) `
  -ExcludeFiles @('*.log')
Write-Output '复制整合包…'
Copy-Tree $Client (Join-Path $kit 'mac-client\.minecraft') -ExcludeDirs @((Join-Path $Client 'runtime'), '*-natives')

$readme = @"
Galaxy宝可梦 Mac 版打包材料

在 Mac 上：
1. 准备工具（只需一次）：Xcode 命令行工具、Rust、Node.js 20+（corepack enable）、JDK 21，详见 source/GALAXY_MAC.md。
2. 打开「终端」，进入解压后的 source 文件夹，运行：
   bash galaxy-build-mac.sh ../mac-client/.minecraft
3. 生成的安装包在 source/dist-mac/ 里，把 dmg 发给玩家。
"@
[IO.File]::WriteAllText((Join-Path $kit '先看我.txt'), $readme, (New-Object Text.UTF8Encoding $true))

$size = (Get-ChildItem -LiteralPath $kit -Recurse -File -Force | Measure-Object Length -Sum).Sum
Write-Output ("文件夹：{0}（{1:N0} MB）" -f $kit, ($size / 1MB))

if (-not $NoZip) {
  Add-Type -AssemblyName System.IO.Compression
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $zip = Join-Path $outDir 'GalaxyPokemon-mac-kit.zip'
  if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
  # 和 make-package.ps1 一样逐个写入：路径用 /，文件名用 UTF-8，Mac 的「归档实用工具」能正确解压中文名
  $fs = [IO.File]::Open($zip, [IO.FileMode]::CreateNew)
  $archive = New-Object IO.Compression.ZipArchive($fs, [IO.Compression.ZipArchiveMode]::Create, $false, [Text.Encoding]::UTF8)
  try {
    foreach ($f in Get-ChildItem -LiteralPath $kit -Recurse -File -Force) {
      $name = $f.FullName.Substring($outDir.Length + 1).Replace('\', '/')
      $level = if ($f.Extension -match '^\.(jar|zip|png|jpg|ogg)$') { [IO.Compression.CompressionLevel]::Fastest } else { [IO.Compression.CompressionLevel]::Optimal }
      $entry = $archive.CreateEntry($name, $level)
      $entry.LastWriteTime = $f.LastWriteTime
      $out = $entry.Open(); $in = [IO.File]::OpenRead($f.FullName)
      try { $in.CopyTo($out) } finally { $in.Dispose(); $out.Dispose() }
    }
  } finally { $archive.Dispose(); $fs.Dispose() }
  Write-Output ("压缩包：{0}（{1:N0} MB）" -f $zip, ((Get-Item -LiteralPath $zip).Length / 1MB))
}
