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

# The uninstaller finds the startup line by this marker. Keep it the same in uninstall.sh.
SUMMARY_MARKER="# aws-billing-info shell summary"

# Print the rc file of the user's login shell. Print nothing if the shell is not supported.
shell_rc_file() {
  platform=$1
  case "$(basename "${SHELL:-}")" in
    bash)
      # On macOS, Terminal starts login shells, which read ~/.bash_profile and not ~/.bashrc.
      if [ "$platform" = "macos" ]; then
        echo "$HOME/.bash_profile"
      else
        echo "$HOME/.bashrc"
      fi
      ;;
    zsh)  echo "${ZDOTDIR:-$HOME}/.zshrc" ;;
    fish) echo "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish" ;;
  esac
}

# Ask the user to show a billing summary when a shell starts.
# Set AWS_BILLING_SUMMARY=yes or AWS_BILLING_SUMMARY=no to skip the question.
setup_shell_summary() {
  platform=$1
  rc_file=$(shell_rc_file "$platform")

  if [ -z "$rc_file" ]; then
    echo "" >&2
    echo "Shell summary: shell '${SHELL:-unknown}' is not supported. See the README to set it up." >&2
    return 0
  fi

  if [ -f "$rc_file" ] && grep -qF "$SUMMARY_MARKER" "$rc_file"; then
    echo "Shell summary: already set up in $rc_file" >&2
    return 0
  fi

  answer="${AWS_BILLING_SUMMARY:-}"
  if [ -z "$answer" ]; then
    echo "" >&2
    echo "A billing summary at shell startup calls AWS Cost Explorer (\$0.01 per call, cached for 6 hours)." >&2
    printf "Show the AWS billing summary when a shell starts? Adds 1 line to %s [y/N] " "$rc_file" >&2
    # The script can run from 'curl | sh', so stdin is the script. Read the answer from the terminal.
    { read -r answer < /dev/tty; } 2>/dev/null || answer=""
  fi

  case "$answer" in
    y|Y|yes|YES|Yes) ;;
    *)
      echo "Shell summary: skipped. See the README to set it up later." >&2
      return 0
      ;;
  esac

  bin="$INSTALL_DIR/$BIN_NAME"
  case "$rc_file" in
    *.fish)
      line="status is-interactive; and test -x \"$bin\"; and \"$bin\" --summary 2>/dev/null $SUMMARY_MARKER"
      ;;
    *)
      line="[[ \$- == *i* ]] && [ -x \"$bin\" ] && \"$bin\" --summary 2>/dev/null $SUMMARY_MARKER"
      ;;
  esac

  mkdir -p "$(dirname "$rc_file")"
  printf '\n%s\n' "$line" >> "$rc_file"
  echo "Shell summary: added to $rc_file. It shows in new shells." >&2
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
  setup_shell_summary "$platform"

  echo "" >&2
  echo "Run '$BIN_NAME --help' to get started." >&2
}

main
