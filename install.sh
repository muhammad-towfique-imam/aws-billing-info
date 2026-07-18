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

get_latest_tag() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
}

download_binary() {
  platform=$1
  arch=$2
  tag=$3
  tmpdir=$(mktemp -d)
  cd "$tmpdir"

  if [ "$platform" = "macos" ]; then
    asset_name="aws-billing-info-${arch}-macos"
  else
    asset_name="aws-billing-info-${arch}-linux"
  fi

  url="https://github.com/${REPO}/releases/download/${tag}/${asset_name}"

  echo "Downloading ${asset_name} (${tag})..."
  curl -fsSL "$url" -o "$BIN_NAME"

  chmod +x "$BIN_NAME"
  echo "$tmpdir/$BIN_NAME"
}

install_binary() {
  binary_path=$1

  mkdir -p "$INSTALL_DIR"

  if [ ! -w "$INSTALL_DIR" ]; then
    echo "Need write permission for $INSTALL_DIR. Try: sudo env INSTALL_DIR=/usr/local/bin $0"
    exit 1
  fi

  cp "$binary_path" "$INSTALL_DIR/$BIN_NAME"
  echo "Installed $BIN_NAME to $INSTALL_DIR"
}

add_to_path() {
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
      echo ""
      echo "Add $INSTALL_DIR to your PATH:"
      echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
      echo ""
      echo "Or add it to your shell rc file:"
      echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc"
      echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc"
      ;;
  esac
}

main() {
  read -r platform arch <<EOF
$(detect_platform)
EOF

  tag=$(get_latest_tag)
  if [ -z "$tag" ]; then
    echo "Failed to determine latest release tag"
    exit 1
  fi

  echo "Platform: $platform ($arch)"
  echo "Latest release: $tag"
  echo "Install dir: $INSTALL_DIR"

  binary_path=$(download_binary "$platform" "$arch" "$tag")
  install_binary "$binary_path"
  add_to_path

  echo ""
  echo "Run '$BIN_NAME --help' to get started."
}

main
