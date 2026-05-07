#!/usr/bin/env bash
set -euo pipefail

REPO="finalquest/strand"
TOOL_NAME="strand"
INSTALL_DIR="${STRAND_INSTALL_DIR:-$HOME/.local/bin}"

echo "Installing ${TOOL_NAME}..."

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}:${ARCH}" in
  Darwin:arm64)
    ASSET="${TOOL_NAME}-macos-arm64"
    ;;
  Linux:x86_64)
    ASSET="${TOOL_NAME}-linux-x86_64"
    ;;
  *)
    echo "Unsupported platform: ${OS} ${ARCH}"
    exit 1
    ;;
esac

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Downloading ${ASSET}..."
# GitHub releases are public, no auth needed
curl -fsSL \
  "https://github.com/${REPO}/releases/latest/download/${ASSET}" \
  -o "${TMP_DIR}/${ASSET}"

mkdir -p "${INSTALL_DIR}"

install -m 0755 \
  "${TMP_DIR}/${ASSET}" \
  "${INSTALL_DIR}/${TOOL_NAME}"

echo ""
echo "${TOOL_NAME} installed to ${INSTALL_DIR}/${TOOL_NAME}"

if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    echo ""
    echo "WARNING: ${INSTALL_DIR} is not in your PATH."
    echo "Add the following to your shell profile:"
    echo ""
    echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
fi

echo "Run '${TOOL_NAME} --help' to get started."
