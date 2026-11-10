# 每个构建 / 开发命令前先 dot-source：  . .\galaxy-env.ps1
# 把所有工具和缓存都指向 D 盘（C 盘只剩不到 3 GB，绝不能让 pnpm / cargo / 临时文件落到 C 盘）
$D = 'D:\GalaxyPokemon\_devtools'
$env:RUSTUP_HOME = "$D\rust\rustup"
$env:CARGO_HOME = "$D\rust\cargo"
$env:CARGO_TARGET_DIR = 'D:\GalaxyPokemon\GalaxyPokemon-Axolotl\target'
$env:TEMP = "$D\tmp"; $env:TMP = "$D\tmp"
New-Item -ItemType Directory -Force $env:TEMP | Out-Null
$env:npm_config_cache = "$D\npm-cache"
$env:npm_config_store_dir = "$D\pnpm-store"
$env:PNPM_HOME = "$D\pnpm-home"
$env:COREPACK_HOME = "$D\corepack"
$env:npm_config_registry = 'https://registry.npmmirror.com'
$env:ELECTRON_MIRROR = 'https://npmmirror.com/mirrors/electron/'
# 数据目录由新的 identifier（com.galaxypokemon.launcher）决定
# 皮肤编辑器（blockbench 子模块）构建很慢且非必需，开发期跳过
$env:AXOLOTL_SKIP_BLOCKBENCH_BUILD = '1'
# JDK（app-lib 构建会编译几个 Java 小工具）
$env:JAVA_HOME = 'D:\GalaxyPokemon\GalaxyPokemon登录服-1.1\runtime\jdk-21.0.12+8'
$env:Path = "$env:CARGO_HOME\bin;$env:PNPM_HOME;$D\node-v26.13.0-win-x64;$env:JAVA_HOME\bin;$env:Path"
# MSVC：加载 VS Build Tools 的开发环境（cl / link / Windows SDK）
$vsDev = 'D:\GalaxyPokemon\_devtools\BuildTools\Common7\Tools\Launch-VsDevShell.ps1'
if ((Test-Path $vsDev) -and -not $env:VSCMD_VER) { & $vsDev -Arch amd64 -HostArch amd64 -SkipAutomaticLocation | Out-Null }

# 种子地图 DLL 用 MinGW gcc 编译；Gradle 缓存与阿里云镜像配置放 D 盘
$env:AXOLOTL_CUBIOMES_CC = 'D:\GalaxyPokemon\_devtools\w64devkit\bin\gcc.exe'
$env:GRADLE_USER_HOME = 'D:\GalaxyPokemon\_devtools\gradle-home'
