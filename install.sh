#!/bin/sh
set -euo pipefail

REPO="muhammad-towfique-imam/aws-billing-info"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="aws-billing-info"

detect_platform() {
  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  arch=$(uname -m)

  case "$os" in
    linux)  os="linux" ;;
    darwin) os="macos" ;;
    *)      echo "Unsupported OS: $os"; exit 1 ;;
  esac

  case "$arch" in
    x86_64)   arch="x86_64" ;;
    aarch64)  arch="aarch64" ;;
    arm64)    arch="aarch64" ;;
    *)        echo "Unsupported arch: $arch"; exit 1 ;;
  esac

  echo "${os} ${arch}"
}

curl_retry() {
  max_retries=3
  retry=0
  until [ $retry -ge $max_retries ]; do
    if curl --http1.1 -fsSL -H "User-Agent: Mozilla/5.0" "$@"; then
      return 0
    fi
    retry=$((retry + 1))
    echo "Retry $retry/$max_retries in 2s..." >&2
    sleep 2
  done
  return 1
}

download_binary() {
  platform=$1
  arch=$2
  tmpdir=$(mktemp -d)
  cd "$tmpdir"

  if [ "$platform" = "macos" ]; then
    asset_name="aws-billing-info-${arch}-macos"
  else
    asset_name="aws-billing-info-${arch}-linux"
  fi

  echo "Downloading ${asset_name}..." >&2

  url="https://github.com/${REPO}/releases/latest/download/${asset_name}"
  echo "Trying: $url" >&2

  if curl_retry -o "$BIN_NAME" "$url"; then
    echo "Downloaded successfully" >&2
    chmod +x "$BIN_NAME"
    echo "$tmpdir/$BIN_NAME"
    return 0
  fi

  echo "Direct download failed, trying GitHub API..." >&2

  tag=$(curl_retry -H "User-Agent: Mozilla/5.0" "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')

  if [ -z "$tag" ]; then
    echo "Failed to determine latest release tag" >&2
    rm -rf "$tmpdir"
    exit 1
  fi

  api_url="https://github.com/${REPO}/releases/download/${tag}/${asset_name}"
  echo "Trying: $api_url" >&2

  if curl_retry -o "$BIN_NAME" "$api_url"; then
    echo "Downloaded successfully via API" >&2
    chmod +x "$BIN_NAME"
    echo "$tmpdir/$BIN_NAME"
    return 0
  fi

  echo "Failed to download binary from both direct and API URLs" >&2
  rm -rf "$tmpdir"
  exit 1
}

install_binary() {
  binary_path=$1

  mkdir -p "$INSTALL_DIR"

  if [ ! -w "$INSTALL_DIR" ]; then
    echo "Need write permission for $INSTALL_DIR. Try: sudo env INSTALL_DIR=/usr/local/bin $0" >&2
    exit 1
  fi

  cp "$binary_path" "$INSTALL_DIR/$BIN_NAME"
  echo "Installed $BIN_NAME to $INSTALL_DIR" >&2
}

add_to_path() {
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
  echo "" >&2
  echo "Add $INSTALL_DIR to your PATH:" >&2
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\"" >&2
  echo "" >&2
  echo "Or add it to your shell rc file:" >&2
  echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc" >&2
  echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc" >&2
      ;;
  esac
}

main() {
  read -r platform arch <<EOF
$(detect_platform)
EOF

  echo "Platform: $platform ($arch)" >&2
  echo "Install dir: $INSTALL_DIR" >&2

  binary_path=$(download_binary "$platform" "$arch")
  install_binary "$binary_path"
  add_to_path

  echo "" >&2
  echo "Run '$BIN_NAME --help' to get started." >&2
}

main
