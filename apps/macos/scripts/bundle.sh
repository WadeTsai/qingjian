#!/usr/bin/env bash
# 把 qingjian-macos 打包成 Qingjian.app。
#
#   scripts/bundle.sh            # 只打包到 target/Qingjian.app
#   scripts/bundle.sh --install  # 打包並安裝到 ~/Library/Input Methods/，殺掉舊程序（開發用）
#   scripts/bundle.sh --pkg      # 打包並做成 target/pkg/qingjian-<版本>-macos-<arm64|x86_64>.pkg（分發給測試者）
#
# 架構：預設編譯本機架構；QINGJIAN_TARGET=x86_64-apple-darwin（或 aarch64-apple-darwin）交叉編譯另一種，
# 先 `rustup target add` 一次。CI 在 Apple Silicon runner 上兩個都打（.github/workflows/release.yml）。
# 產品資料：data/generated/ 裡有 dict.qj 就用自建詞庫（TSV 比 .qj 新會重打），沒有就退回 assets/sample/ 樣例。
#
# 簽名與公證都由環境變數決定，沒設就 ad-hoc 簽名、pkg 不籤（本機自用夠了，分發給別人會被 Gatekeeper 攔，
# 對方要在「系統設定 → 隱私與安全性」裡點「仍要開啟」）：
#   QINGJIAN_SIGN_IDENTITY       "Developer ID Application: …"   給 .app 簽名（hardened runtime）
#   QINGJIAN_INSTALLER_IDENTITY  "Developer ID Installer: …"     給 .pkg 簽名
#   QINGJIAN_NOTARY_PROFILE      notarytool store-credentials 存的 keychain profile 名，設了就公證並釘票據
#
# 首次 --install 後要在「系統設定 → 鍵盤 → 輸入法」裡新增「青簡」；輸入法列表不重新整理就登出再登入。
# pkg 裝的不用：postinstall 會以登入使用者身份跑 `qingjian-macos --register` 註冊並啟用。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
APP_NAME="Qingjian"
BIN_NAME="qingjian-macos"
PROFILE="${PROFILE:-release}"
APP="$ROOT/target/$APP_NAME.app"
INSTALL_DIR="$HOME/Library/Input Methods"
# 目標三元組為空就是本機；架構名按 pkg 檔名與 distribution.xml 的 hostArchitectures 用的寫法（arm64 / x86_64）
TARGET="${QINGJIAN_TARGET:-}"
case "${TARGET:-$(uname -m)}" in
  aarch64-apple-darwin|arm64) ARCH="arm64" ;;
  x86_64-apple-darwin|x86_64) ARCH="x86_64" ;;
  *) echo "不认识的架构: ${TARGET:-$(uname -m)}" >&2; exit 1 ;;
esac

cd "$ROOT"
# 構建標識進「關於」頁與診斷資訊：git 短雜湊（工作區有改動加 +）與日期；編譯期 option_env! 讀
GIT_REV="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then GIT_REV="${GIT_REV}+"; fi
export QINGJIAN_BUILD="${GIT_REV} · $(date +%Y-%m-%d)"
BUILD_ARGS=(-p "$BIN_NAME" --locked)
[[ "$PROFILE" == "release" ]] && BUILD_ARGS+=(--release)
BIN_DIR="target/$PROFILE"
if [[ -n "$TARGET" ]]; then
  BUILD_ARGS+=(--target "$TARGET")
  BIN_DIR="target/$TARGET/$PROFILE"
fi
cargo build "${BUILD_ARGS[@]}"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN_DIR/$BIN_NAME" "$APP/Contents/MacOS/$BIN_NAME"
cp apps/macos/Info.plist "$APP/Contents/Info.plist"
# 版本號來自 apps/macos/Cargo.toml（各平臺殼版本號獨立，不跟 workspace 走），構建號用提交數（單調遞增，pkg 升級判斷靠它）。
# 發版之間版本號帶 -dev（0.1.2-dev）：本地與 CI 中間構建一眼能與線上包區分；發版提交去掉 -dev 再打標籤（docs/notes/release.md）。
# 開發版再接上 git 短雜湊（0.1.3-dev-1a2b3c4，工作區有改動加 +），測試時一眼知道裝的是哪個提交；Cargo.toml 裡仍只寫 -dev。
# pkgbuild / distribution 的 version 只認數字點號，去掉預釋出字尾；Info.plist 與 pkg 檔名保留完整版本
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' apps/macos/Cargo.toml | head -1)"
if [[ "$VERSION" == *-dev ]]; then VERSION="${VERSION}-${GIT_REV}"; fi
PKG_VERSION="${VERSION%%-*}"
BUILD_NUMBER="$(git rev-list --count HEAD 2>/dev/null || echo 1)"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" \
  -c "Set :CFBundleVersion $BUILD_NUMBER" "$APP/Contents/Info.plist"
# 解除安裝指令碼隨包，裝了 pkg 的使用者從 Resources 裡執行
cp apps/macos/scripts/uninstall.sh "$APP/Contents/Resources/uninstall.sh"
# 輸入源名字按系統語言本地化（中文系統顯示「青簡」，其他顯示 Qingjian）
cp -R apps/macos/resources/*.lproj "$APP/Contents/Resources/"
# 詞庫與釋義表打進 Resources。data/generated/ 裡有生成好的產品資料（自建詞庫 + 語言模型 + LLM 釋義表）就用它，
# 否則用 assets/sample/ 的樣例。沒有資料管道的機器跑 tools/release/data-fetch.sh 按 tools/release/data.lock 下載。
cp assets/sample/*.tsv "$APP/Contents/Resources/"
# emoji 表（Unicode CLDR，可釋出）
cp assets/emoji/*.tsv "$APP/Contents/Resources/"
# 詞彙等級表（CEFR-J / Octanove / JLPT，見 assets/levels/README.md），「統計」頁按級數詞彙
cp assets/levels/levels-*.tsv "$APP/Contents/Resources/"
# 五筆碼錶（輸入方案選五筆時用，見 assets/wubi/README.md；極點 86 碼錶，Apache-2.0）
mkdir -p "$APP/Contents/Resources/wubi"
cp assets/wubi/wubi86.tsv "$APP/Contents/Resources/wubi/"
if [[ -f data/generated/dict.tsv || -f data/generated/dict.qj ]]; then
  # 詞庫與語言模型打成 .qj（mmap 直接用），TSV 比 .qj 新時重新打包；只有 .qj（CI 從資料包解出來的）就直接用
  if [[ -f data/generated/dict.tsv && ( ! -f data/generated/dict.qj || data/generated/dict.tsv -nt data/generated/dict.qj ) ]]; then
    cargo run --release -q -p qingjian-dict-convert -- pack dict --name "青简基础词库" \
      --license "MIT AND Unicode-3.0" --attribution "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；读音 Unihan（Unicode）" \
      --source https://github.com/qingjian-team/qingjian/tree/main/assets/lexicon
  fi
  if [[ -f data/generated/lm-bigram.tsv && ( ! -f data/generated/lm.qj || data/generated/lm-bigram.tsv -nt data/generated/lm.qj ) ]]; then
    cargo run --release -q -p qingjian-dict-convert -- pack lm --name "青简语言模型（中文维基 + LCCC，青简词库分词）" \
      --license "CC-BY-SA-4.0 AND MIT" --attribution "中文维基百科（CC BY-SA 4.0）；LCCC（清华大学 CoAI，MIT）"
  fi
  cp data/generated/dict.qj "$APP/Contents/Resources/"
  # 領域詞庫（lexicon 拆出的 dicts/*.qj）隨包放 Resources/dicts/，預設只開成語，偏好設定「詞庫」頁可勾選
  if ls data/generated/dicts/*.qj >/dev/null 2>&1; then
    mkdir -p "$APP/Contents/Resources/dicts"
    cp data/generated/dicts/*.qj "$APP/Contents/Resources/dicts/"
  fi
  [[ -f data/generated/lm.qj ]] && cp data/generated/lm.qj "$APP/Contents/Resources/"
  # 含章·知微（字級 Transformer）：三件套與單檔案放 data/models/hanzhang-zhiwei/。
  model_dir="${QINGJIAN_MODEL_DIR:-data/models/hanzhang-zhiwei}"
  if [[ -f "$model_dir/model.safetensors" ]]; then
    QINGJIAN_MODEL_DIR="$model_dir" tools/release/pack-model.sh
  fi
  if [[ -f "$model_dir/hanzhang-zhiwei-small.qjm" ]]; then
    mkdir -p "$APP/Contents/Resources/models/hanzhang-zhiwei"
    cp "$model_dir/hanzhang-zhiwei-small.qjm" "$APP/Contents/Resources/models/hanzhang-zhiwei/"
    chmod 644 "$APP/Contents/Resources/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm"
    echo "打包含章·知微：$model_dir/hanzhang-zhiwei-small.qjm"
  fi
  # 含章·通變（P2C，帶噪拼音 → 漢字）：放 Resources/models/hanzhang-tongbian/。
  p2c_dir="${QINGJIAN_P2C_MODEL_DIR:-data/models/hanzhang-tongbian}"
  if [[ -f "$p2c_dir/model.safetensors" ]]; then
    QINGJIAN_MODEL_DIR="$p2c_dir" tools/release/pack-model.sh
  fi
  if [[ -f "$p2c_dir/hanzhang-tongbian-small.qjm" ]]; then
    mkdir -p "$APP/Contents/Resources/models/hanzhang-tongbian"
    cp "$p2c_dir/hanzhang-tongbian-small.qjm" "$APP/Contents/Resources/models/hanzhang-tongbian/"
    chmod 644 "$APP/Contents/Resources/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm"
    echo "打包含章·通变：$p2c_dir/hanzhang-tongbian-small.qjm"
  fi
  # 釋義表打成 .qj（TSV 比 .qj 新時重打），英文詞表仍是 TSV。各表來源不同，後設資料按表寫（見 assets/glossary/README.md）
  for lang in en ja zh es vi; do
    src="assets/glossary/glossary-$lang.tsv"
    out="data/generated/glossary-$lang.qj"
    [[ -f "$src" ]] || continue
    if [[ "$lang" == es ]]; then
      license="GPL-3.0-or-later"
      attribution="Azure Translator 机器翻译（Tofuzhu，tools/corpus/glossary_es.py）"
    elif [[ "$lang" == vi ]]; then
      license="GPL-3.0-or-later"
      attribution="LLM 生成（gpt-5.5），qingjian-gloss-gen"
    else
      license="MIT"
      attribution="LLM 生成（DeepSeek），qingjian-gloss-gen"
    fi
    if [[ ! -f "$out" || "$src" -nt "$out" ]]; then
      cargo run --release -q -p qingjian-dict-convert -- pack glossary --language "$lang" --input "$src" \
        --name "青简释义表（${lang}）" --license "$license" --attribution "$attribution"
    fi
    cp "$out" "$APP/Contents/Resources/"
  done
  for f in assets/lexicon/english.tsv data/generated/english.tsv; do
    [[ -f "$f" ]] && cp "$f" "$APP/Contents/Resources/"
  done
  echo "使用 data/generated/ 的产品数据（自建词库）"
fi
printf 'APPL????' > "$APP/Contents/PkgInfo"

# 圖示：應用圖示從 assets/icon/logo.png 生成 .icns；輸入法選單圖示直接用 assets/icon/menu.pdf（向量，隨 Info.plist 的 TISIconIsTemplate 按深淺色反色）
ICONSET="$ROOT/target/Qingjian.iconset"
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon/logo.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z $double $double assets/icon/logo.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Qingjian.icns"
cp assets/icon/menu.pdf "$APP/Contents/Resources/qingjian-menu.pdf"
# 倉庫放在 iCloud 同步的目錄（Documents）時新建的 .app 會帶上 Finder 擴充套件屬性，codesign 會拒（detritus not allowed）：簽名前清掉
xattr -cr "$APP"
# Apple Silicon 上未簽名的二進位制不會被系統載入。有 Developer ID 證書就正式籤（開 hardened runtime，公證要求），
# 沒有就 ad-hoc 簽名，本機自用夠了
if [[ -n "${QINGJIAN_SIGN_IDENTITY:-}" ]]; then
  codesign --force --deep --options runtime --timestamp --sign "$QINGJIAN_SIGN_IDENTITY" "$APP"
  echo "已用 Developer ID 签名: $QINGJIAN_SIGN_IDENTITY"
else
  codesign --force --deep --sign - "$APP"
fi
echo "打包完成: ${APP}（版本 ${VERSION}，构建 ${BUILD_NUMBER}，${ARCH}）"

if [[ "${1:-}" == "--pkg" ]]; then
  # 每個架構一個工作目錄，成品都放 target/pkg/，兩個架構接著打互不覆蓋
  PKG="$ROOT/target/pkg/qingjian-$VERSION-macos-$ARCH.pkg"
  PKG_DIR="$ROOT/target/pkg/$ARCH"
  rm -rf "$PKG_DIR"
  mkdir -p "$PKG_DIR/root" "$PKG_DIR/resources"
  # 不帶擴充套件屬性複製，否則載荷裡全是 ._ 後設資料檔案
  ditto --noextattr --norsrc --noacl "$APP" "$PKG_DIR/root/$APP_NAME.app"
  # 元件描述裡關掉 bundle 重定位：否則機器上別處已有同 bundle id 的 .app（比如 ~/Library 下的開發副本）時，
  # 安裝器會把新版裝到那裡而不是 /Library/Input Methods
  pkgbuild --analyze --root "$PKG_DIR/root" "$PKG_DIR/component.plist" >/dev/null
  /usr/libexec/PlistBuddy -c "Set :0:BundleIsRelocatable false" "$PKG_DIR/component.plist" 2>/dev/null || \
    /usr/libexec/PlistBuddy -c "Add :0:BundleIsRelocatable bool false" "$PKG_DIR/component.plist"
  pkgbuild --root "$PKG_DIR/root" --component-plist "$PKG_DIR/component.plist" \
    --install-location "/Library/Input Methods" --scripts apps/macos/pkg/scripts \
    --identifier app.qingjian.inputmethod --version "$PKG_VERSION" "$PKG_DIR/$APP_NAME-component.pkg" >/dev/null
  cp apps/macos/pkg/resources/*.html "$PKG_DIR/resources/"
  cp LICENSE "$PKG_DIR/resources/license.txt"
  # 二進位制只有一種架構，hostArchitectures 限定只在對應機器上裝；另一種架構用 QINGJIAN_TARGET 再打一份
  sed -e "s/@VERSION@/$VERSION/g" -e "s/@PKG_VERSION@/$PKG_VERSION/g" -e "s/@ARCH@/$ARCH/g" apps/macos/pkg/distribution.xml > "$PKG_DIR/distribution.xml"
  SIGN_ARGS=()
  if [[ -n "${QINGJIAN_INSTALLER_IDENTITY:-}" ]]; then
    SIGN_ARGS=(--sign "$QINGJIAN_INSTALLER_IDENTITY" --timestamp)
  fi
  # bash 3.2 下空陣列展開會撞 set -u，用 ${arr[@]+"${arr[@]}"} 寫法
  productbuild --distribution "$PKG_DIR/distribution.xml" --package-path "$PKG_DIR" \
    --resources "$PKG_DIR/resources" ${SIGN_ARGS[@]+"${SIGN_ARGS[@]}"} "$PKG" >/dev/null
  if [[ -n "${QINGJIAN_NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$PKG" --keychain-profile "$QINGJIAN_NOTARY_PROFILE" --wait
    xcrun stapler staple "$PKG"
    echo "已公证并钉上票据"
  elif [[ -z "${QINGJIAN_INSTALLER_IDENTITY:-}" ]]; then
    echo "注意: pkg 未签名未公证，测试者首次打开要在「系统设置 → 隐私与安全性」里点「仍要打开」"
  fi
  echo "pkg: $PKG"
  shasum -a 256 "$PKG"
fi

if [[ "${1:-}" == "--install" ]]; then
  if [[ -d "/Library/Input Methods/$APP_NAME.app" ]]; then
    echo "注意: /Library/Input Methods/$APP_NAME.app 也装着一份（pkg 装的），两份同 id 会互相顶；先跑 scripts/uninstall.sh"
  fi
  mkdir -p "$INSTALL_DIR"
  # 先整個拷到旁邊再換上去：直接刪了再拷，拷到一半中斷會留下殘缺的包，舊程序還在用它
  STAGING="$INSTALL_DIR/.$APP_NAME.app.new"
  rm -rf "$STAGING"
  cp -R "$APP" "$STAGING"
  # 系統會在下次切換到該輸入法時重新拉起程序
  pkill -x "$BIN_NAME" 2>/dev/null || true
  rm -rf "$INSTALL_DIR/$APP_NAME.app"
  mv "$STAGING" "$INSTALL_DIR/$APP_NAME.app"
  echo "已安装到: $INSTALL_DIR/$APP_NAME.app"
  echo "日志: ~/Library/Logs/Qingjian/"
fi
