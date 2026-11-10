#!/usr/bin/env bash
# Galaxy宝可梦 Mac 版打包（在 Mac 上运行；Windows 上编不了 Mac 版）
#
# 用法（在源码根目录）：
#   bash galaxy-build-mac.sh <整合包 .minecraft 目录> [版本号]
#   例：bash galaxy-build-mac.sh ../mac-client/.minecraft 1.0.7
# 产物：dist-mac/GalaxyPokemon_v<版本号>_mac.dmg（Apple 芯片和 Intel 通用）
# 准备工作和原理见 GALAXY_MAC.md。
set -euo pipefail
cd "$(dirname "$0")"

GAME_DIR="${1:-}"
VERSION="${2:-}"
API_BASE="${GALAXY_API_BASE:-http://mc9.yunmaoc.com:28095}"
APP_NAME="Galaxy宝可梦"
VERSION_ID="1.21.1-Fabric 0.19.3"

die() { echo "错误：$*" >&2; exit 1; }

[ "$(uname)" = "Darwin" ] || die "只能在 Mac 上运行"
[ -n "$GAME_DIR" ] || die "请传入整合包的 .minecraft 目录，例如：bash galaxy-build-mac.sh ../mac-client/.minecraft"
[ -f "$GAME_DIR/versions/$VERSION_ID/$VERSION_ID.json" ] || die "$GAME_DIR 里没有版本 $VERSION_ID"
for tool in cargo rustup node pnpm; do
  command -v "$tool" >/dev/null 2>&1 || die "缺少 $tool，按 GALAXY_MAC.md 的「准备」安装"
done
xcode-select -p >/dev/null 2>&1 || die "缺少 Xcode 命令行工具，先运行 xcode-select --install"
if [ -z "${JAVA_HOME:-}" ]; then
  JAVA_HOME="$(/usr/libexec/java_home -v 21 2>/dev/null)" || die "缺少 JDK 21（编译时要用），按 GALAXY_MAC.md 的「准备」安装"
  export JAVA_HOME
fi
if [ -z "$VERSION" ]; then
  VERSION="$(node -p "require('./apps/app-frontend/package.json').version")"
fi
VERSION_RE='^[0-9]+(\.[0-9]+){1,3}$'
[[ "$VERSION" =~ $VERSION_RE ]] || die "版本号格式不对：$VERSION（例如 1.0.7）"

export AXOLOTL_SKIP_BLOCKBENCH_BUILD=1
export npm_config_registry="${npm_config_registry:-https://registry.npmmirror.com}"

echo "== 1/5 安装依赖"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm install --frozen-lockfile

echo "== 2/5 编译（通用版：Apple 芯片 + Intel）"
pnpm --filter @modrinth/app exec tauri build --target universal-apple-darwin --bundles app --config tauri-release.conf.json
APP="target/universal-apple-darwin/release/bundle/macos/$APP_NAME.app"
[ -d "$APP" ] || die "没找到编译结果：$APP"

echo "== 3/5 放入整合包和版本信息"
RES="$APP/Contents/Resources"
rm -rf "$RES/.minecraft"
# Windows 的 Java（runtime/）和本地库（*-natives）在 Mac 上用不了；Mac 版的会在第一次启动时自动下载
rsync -a --exclude '/runtime/' --exclude '*-natives/' --exclude '.DS_Store' "$GAME_DIR/" "$RES/.minecraft/"
cat > "$RES/galaxy-launcher.json" <<EOF
{
  "version": "$VERSION",
  "built": "$(date +%Y-%m-%dT%H:%M:%S)",
  "apiBase": "$API_BASE"
}
EOF

echo "== 4/5 签名（ad-hoc，没有苹果开发者证书）"
xattr -cr "$APP"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

echo "== 5/5 生成 dmg"
OUT="dist-mac"
STAGE="$OUT/stage"
DMG="$OUT/GalaxyPokemon_v${VERSION}_mac.dmg"
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/$APP_NAME.app"
ln -s /Applications "$STAGE/Applications"
cp GALAXY_MAC_PLAYERS.txt "$STAGE/打不开时看这里.txt"
# GitHub 的 Mac 机器上 hdiutil 偶尔报 "Resource busy"，重试几次
for i in 1 2 3 4 5; do
  hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" && break
  [ "$i" -lt 5 ] || die "hdiutil 生成 dmg 失败"
  echo "hdiutil 失败，15 秒后重试（第 $i 次）"
  sleep 15
done
rm -rf "$STAGE"
echo "完成：$DMG（$(du -h "$DMG" | cut -f1)）"
