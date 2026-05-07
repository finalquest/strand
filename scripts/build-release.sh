#!/usr/bin/env bash
set -euo pipefail

# Build release binaries for strand
# macOS ARM64: native cargo build
# Linux x86_64: Docker buildx

VERSION="$(cargo metadata --format-version=1 --no-deps | sed -n 's/.*"version":"\([^"]*\)".*/\1/p')"
DIST_DIR="dist"
BINARY_NAME="strand"

echo "Building strand v${VERSION}..."
echo ""

mkdir -p "${DIST_DIR}"

# Detect OS
OS="$(uname -s)"
ARCH="$(uname -m)"

# --- macOS ARM64 ---
if [[ "${OS}" == "Darwin" && "${ARCH}" == "arm64" ]]; then
    echo "Building macOS ARM64 binary..."
    cargo build --release
    cp "target/release/${BINARY_NAME}" "${DIST_DIR}/${BINARY_NAME}-macos-arm64"
    echo "  -> ${DIST_DIR}/${BINARY_NAME}-macos-arm64"
else
    echo "Skipping macOS ARM64 build (requires macOS ARM64 host)"
fi

echo ""

# --- Linux x86_64 ---
echo "Building Linux x86_64 binary via Docker buildx..."
docker buildx build --platform linux/amd64 -f scripts/Dockerfile.linux -t strand-linux-builder --load .

# Extract binary from image
CONTAINER_ID="$(docker create strand-linux-builder)"
docker cp "${CONTAINER_ID}:/app/target/release/${BINARY_NAME}" "${DIST_DIR}/${BINARY_NAME}-linux-x86_64"
docker rm "${CONTAINER_ID}" >/dev/null

echo "  -> ${DIST_DIR}/${BINARY_NAME}-linux-x86_64"

echo ""
echo "Done. Release artifacts:"
ls -la "${DIST_DIR}/"

echo ""
echo "Creating GitHub release v${VERSION}..."
gh release create "v${VERSION}" \
  "${DIST_DIR}/${BINARY_NAME}-macos-arm64" \
  "${DIST_DIR}/${BINARY_NAME}-linux-x86_64" \
  --title "strand v${VERSION}" \
  --notes "Release v${VERSION}"
