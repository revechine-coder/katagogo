#!/bin/sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RELEASE_DIR="${ROOT_DIR}/releases/arm64"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/katagogo-arm64.XXXXXX")"
ARCHIVE_PATH="${TMP_DIR}/KataGoGo-arm64.xcarchive"
APP_PATH="${ARCHIVE_PATH}/Products/Applications/KataGoGo.app"
ZIP_PATH="${RELEASE_DIR}/KataGoGo-arm64.app.zip"

mkdir -p "${RELEASE_DIR}"

echo "==> Building go_core for arm64"
cd "${ROOT_DIR}/go-core"
cargo build --release --target aarch64-apple-darwin
ditto "${ROOT_DIR}/go-core/target/aarch64-apple-darwin/release/libgo_core.a" \
  "${ROOT_DIR}/KataGoGo/SharedCore/libgo_core.a"
ditto "${ROOT_DIR}/go-core/src/go_core.h" \
  "${ROOT_DIR}/KataGoGo/SharedCore/go_core.h"

echo "==> Checking bundled arm64 engine"
lipo -info "${ROOT_DIR}/kata-engine/katago-eigen" | grep -q "arm64"

echo "==> Archiving arm64 app"
cd "${ROOT_DIR}"
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
xcodebuild -quiet \
  -project KataGoGo.xcodeproj \
  -scheme KataGoGo \
  -configuration Release \
  -derivedDataPath /tmp/KataGoGoArm64DerivedData \
  -archivePath "${ARCHIVE_PATH}" \
  archive ARCHS=arm64 ONLY_ACTIVE_ARCH=NO MACOSX_DEPLOYMENT_TARGET=12.0 ENABLE_USER_SCRIPT_SANDBOXING=NO

echo "==> Verifying arm64 archive"
lipo -info "${APP_PATH}/Contents/MacOS/KataGoGo" | grep -q "arm64"
lipo -info "${APP_PATH}/Contents/Resources/kata-engine/katago-eigen" | grep -q "arm64"
codesign --verify --deep --strict --verbose=2 "${APP_PATH}"

echo "==> Packaging ${ZIP_PATH}"
ditto -c -k --sequesterRsrc --keepParent "${APP_PATH}" "${ZIP_PATH}"
(
  cd "${RELEASE_DIR}"
  shasum -a 256 "$(basename "${ZIP_PATH}")" > SHA256SUMS
)

cat > "${RELEASE_DIR}/README.md" <<EOF
# KataGoGo arm64 packaged build

Artifact:

- \`KataGoGo-arm64.app.zip\`

Compatibility:

- Architecture: \`arm64\`
- Minimum macOS version: \`12.0\`
- Target Macs: Apple Silicon

Bundled engine:

- \`katago-eigen\`
- \`gtp.cfg\`
- \`kata1-b18c384nbt.bin.gz\`

Verification:

- App executable is \`arm64\`
- Bundled engine is \`arm64\`
- \`codesign --verify --deep --strict\` passed
- SHA256 is recorded in \`SHA256SUMS\`
EOF

echo "==> Done: ${ZIP_PATH}"
