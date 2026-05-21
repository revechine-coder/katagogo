#!/bin/sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RELEASE_DIR="${ROOT_DIR}/releases/intel-macos12"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/katagogo-intel.XXXXXX")"
ARCHIVE_PATH="${TMP_DIR}/KataGoGo-Intel-macOS12.xcarchive"
APP_PATH="${ARCHIVE_PATH}/Products/Applications/KataGoGo.app"
ZIP_PATH="${RELEASE_DIR}/KataGoGo-Intel-macOS12.app.zip"
SAVED_LIB="${TMP_DIR}/libgo_core.original.a"
SAVED_ENGINE="${TMP_DIR}/katago-eigen.original"
KATAGO_BUILD_DIR="${TMP_DIR}/katago-eigen-x86_64"

restore_workspace_binaries() {
  if [ -f "${SAVED_LIB}" ]; then
    ditto "${SAVED_LIB}" "${ROOT_DIR}/KataGoGo/SharedCore/libgo_core.a"
  fi
  if [ -f "${SAVED_ENGINE}" ]; then
    ditto "${SAVED_ENGINE}" "${ROOT_DIR}/kata-engine/katago-eigen"
  fi
}
trap restore_workspace_binaries EXIT

mkdir -p "${RELEASE_DIR}"
ditto "${ROOT_DIR}/KataGoGo/SharedCore/libgo_core.a" "${SAVED_LIB}"
ditto "${ROOT_DIR}/kata-engine/katago-eigen" "${SAVED_ENGINE}"

if ! rustup target list --installed | grep -q '^x86_64-apple-darwin$'; then
  echo "error: missing Rust target x86_64-apple-darwin. Run: rustup target add x86_64-apple-darwin" >&2
  exit 1
fi

if [ ! -d /opt/homebrew/include/eigen3 ]; then
  echo "error: missing Eigen headers at /opt/homebrew/include/eigen3" >&2
  exit 1
fi

echo "==> Building go_core for Intel x86_64"
cd "${ROOT_DIR}/go-core"
cargo build --release --target x86_64-apple-darwin
ditto "${ROOT_DIR}/go-core/target/x86_64-apple-darwin/release/libgo_core.a" \
  "${ROOT_DIR}/KataGoGo/SharedCore/libgo_core.a"
ditto "${ROOT_DIR}/go-core/src/go_core.h" \
  "${ROOT_DIR}/KataGoGo/SharedCore/go_core.h"

echo "==> Building KataGo Eigen for Intel x86_64 / macOS 12"
cmake -S "${ROOT_DIR}/kata-go-src/cpp" \
  -B "${KATAGO_BUILD_DIR}" \
  -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DUSE_BACKEND=EIGEN \
  -DUSE_AVX2=1 \
  -DBUILD_DISTRIBUTED=0 \
  -DCMAKE_OSX_ARCHITECTURES=x86_64 \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=12.0 \
  -DEIGEN3_INCLUDE_DIRS=/opt/homebrew/include/eigen3 \
  -DLIBZIP_LIBRARY=LIBZIP_LIBRARY-NOTFOUND \
  -DLIBZIP_INCLUDE_DIR_ZIP=LIBZIP_INCLUDE_DIR_ZIP-NOTFOUND \
  -DLIBZIP_INCLUDE_DIR_ZIPCONF=LIBZIP_INCLUDE_DIR_ZIPCONF-NOTFOUND
cmake --build "${KATAGO_BUILD_DIR}" --target katago --config Release
ditto "${KATAGO_BUILD_DIR}/katago" "${ROOT_DIR}/kata-engine/katago-eigen"

echo "==> Archiving Intel macOS 12 app"
cd "${ROOT_DIR}"
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
xcodebuild -quiet \
  -project KataGoGo.xcodeproj \
  -scheme KataGoGo \
  -configuration Release \
  -derivedDataPath /tmp/KataGoGoIntelDerivedData \
  -archivePath "${ARCHIVE_PATH}" \
  archive ARCHS=x86_64 ONLY_ACTIVE_ARCH=NO MACOSX_DEPLOYMENT_TARGET=12.0 ENABLE_USER_SCRIPT_SANDBOXING=NO

echo "==> Removing Apple Silicon Homebrew dylibs from Intel package"
rm -f "${APP_PATH}/Contents/Resources/kata-engine/libzip.5.dylib" \
      "${APP_PATH}/Contents/Resources/kata-engine/liblzma.5.dylib" \
      "${APP_PATH}/Contents/Resources/kata-engine/libzstd.1.dylib"
codesign --force --deep --sign - "${APP_PATH}"

echo "==> Verifying Intel archive"
lipo -info "${APP_PATH}/Contents/MacOS/KataGoGo" | grep -q "x86_64"
lipo -info "${APP_PATH}/Contents/Resources/kata-engine/katago-eigen" | grep -q "x86_64"
vtool -show-build "${APP_PATH}/Contents/MacOS/KataGoGo" | grep -q "minos 12.0"
vtool -show-build "${APP_PATH}/Contents/Resources/kata-engine/katago-eigen" | grep -q "minos 12.0"
codesign --verify --deep --strict --verbose=2 "${APP_PATH}"

echo "==> Smoke testing Intel engine under Rosetta"
arch -x86_64 "${APP_PATH}/Contents/Resources/kata-engine/katago-eigen" gtp \
  -config "${APP_PATH}/Contents/Resources/kata-engine/gtp.cfg" \
  -model "${APP_PATH}/Contents/Resources/kata-engine/kata1-b18c384nbt.bin.gz" \
  < /dev/null 2>&1 | grep -q "GTP ready"

echo "==> Packaging ${ZIP_PATH}"
ditto -c -k --sequesterRsrc --keepParent "${APP_PATH}" "${ZIP_PATH}"
(
  cd "${RELEASE_DIR}"
  shasum -a 256 "$(basename "${ZIP_PATH}")" > SHA256SUMS
)

cat > "${RELEASE_DIR}/README.md" <<EOF
# KataGoGo Intel macOS 12 packaged build

Artifact:

- \`KataGoGo-Intel-macOS12.app.zip\`

Compatibility:

- Architecture: \`x86_64\`
- Minimum macOS version: \`12.0\`
- Target Macs: Intel Macs, including 2016 MacBook Pro

Bundled engine:

- \`katago-eigen\`
- \`gtp.cfg\`
- \`kata1-b18c384nbt.bin.gz\`

Verification:

- App executable is \`x86_64\`
- Bundled engine is \`x86_64\`
- Both app and engine report \`minos 12.0\`
- Bundled engine reached \`GTP ready\` under \`arch -x86_64\`
- \`codesign --verify --deep --strict\` passed
- SHA256 is recorded in \`SHA256SUMS\`
EOF

echo "==> Done: ${ZIP_PATH}"
