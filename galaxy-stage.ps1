# Galaxy宝可梦：把 tauri build 的输出整理到 dist-launcher\（make-package.ps1 和 dfs-release.ps1 都从这里取）
#
#   . .\galaxy-env.ps1
#   cmd /c "pnpm --filter @modrinth/app exec tauri build --config tauri-release.conf.json --no-bundle"
#   powershell -ExecutionPolicy Bypass -File galaxy-stage.ps1 [-Version 1.0.1]
#
# 版本号默认取 D:\GalaxyPokemon\GalaxyPokemon-Launcher\package.json 的 version（安装包也用它命名）。
# 写进 dist-launcher\galaxy-launcher.json；启动器靠这个文件判断自己的版本，没有它就不会自动更新。
# -ApiBase：服务器数据接口（GalaxyLauncherApi 插件，GALAXY_LAUNCHER_API.md）的公网地址，也写进这个文件。
#   传空字符串（-ApiBase ''）就不写，启动器里依赖它的功能（玩家数据、活动、实时公告、崩溃上传）会隐藏。
param(
  [string]$Version = '',
  [string]$ApiBase = 'http://mc9.yunmaoc.com:28095'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $Version) {
  $pkg = [IO.File]::ReadAllText('D:\GalaxyPokemon\GalaxyPokemon-Launcher\package.json', [Text.Encoding]::UTF8) | ConvertFrom-Json
  $Version = $pkg.version
}
if ($Version -notmatch '^\d+(\.\d+){1,3}$') { throw "版本号格式不对：$Version（例如 1.0.1）" }

$release = Join-Path $root 'target\release'
$exe = Join-Path $release 'GalaxyPokemon.exe'
if (-not (Test-Path $exe)) { throw "找不到 $exe，请先运行 tauri build" }
$out = Join-Path $root 'dist-launcher'
if (Test-Path $out) { Remove-Item -LiteralPath $out -Recurse -Force }
New-Item -ItemType Directory $out | Out-Null

Copy-Item $exe $out
# 瘦身第 2 阶段起 tauri 配置里没有 resources（皮肤编辑器、种子地图已去掉），target\release\resources 可能不存在
$resources = Join-Path $release 'resources'
if (Test-Path $resources) {
  robocopy $resources (Join-Path $out 'resources') /E /NFL /NDL /NJH /NJS /NP /R:1 /W:1 | Out-Null
  if ($LASTEXITCODE -ge 8) { throw "复制 resources 失败（robocopy $LASTEXITCODE）" }
}
# GPL 许可文件（和上游发布包一致）
Copy-Item (Join-Path $root 'apps\app\LICENSE') (Join-Path $out 'LICENSE.txt')
Copy-Item (Join-Path $root 'apps\app\COPYING.md') (Join-Path $out 'COPYING.md')
Copy-Item (Join-Path $root 'COPYING.md') (Join-Path $out 'COPYING-repo.md')

$info = [ordered]@{ version = $Version; built = (Get-Date).ToString('s') }
if ($ApiBase) {
  if ($ApiBase -notmatch '^https?://[^/\s?#@]+(/[^\s?#]*)?$') { throw "数据接口地址格式不对：$ApiBase（例如 http://mc9.yunmaoc.com:28095）" }
  $info.apiBase = $ApiBase.TrimEnd('/')
}
$info = $info | ConvertTo-Json
[IO.File]::WriteAllText((Join-Path $out 'galaxy-launcher.json'), $info, (New-Object Text.UTF8Encoding $false))
Write-Output "已整理到 $out（启动器版本 $Version，数据接口 $(if ($ApiBase) { $ApiBase } else { '未配置' })）"
