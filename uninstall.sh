#!/bin/sh
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="aws-billing-info"
# install.sh writes this marker on the shell startup line. Keep it the same in both scripts.
SUMMARY_MARKER="# aws-billing-info shell summary"

RC_FILES="$HOME/.bashrc
$HOME/.bash_profile
$HOME/.profile
${ZDOTDIR:-$HOME}/.zshrc
${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"

# Remove each line that contains the fixed string $2 from the file $1.
# Use grep -F, not sed: the string contains '/', '$' and '[' characters.
remove_lines() {
  file=$1
  text=$2
  [ -f "$file" ] || return 0
  grep -qF "$text" "$file" || return 0

  tmp=$(mktemp)
  grep -vF "$text" "$file" > "$tmp" || true
  # Write back with 'cat >' to keep the permissions and symlinks of the file.
  cat "$tmp" > "$file"
  rm -f "$tmp"
  echo "Removed the shell summary line from $file"
}

if [ ! -f "$INSTALL_DIR/$BIN_NAME" ]; then
  echo "$BIN_NAME not found in $INSTALL_DIR" >&2
else
  rm -f "$INSTALL_DIR/$BIN_NAME"
  echo "Removed $BIN_NAME from $INSTALL_DIR"
fi

echo "$RC_FILES" | while IFS= read -r rc; do
  remove_lines "$rc" "$SUMMARY_MARKER"
done

# Remove the summary cache (Linux and macOS locations).
rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/aws-billing-info" "$HOME/Library/Caches/aws-billing-info"

echo "Uninstall complete. Restart your shell or run 'hash -r' if $BIN_NAME is still in your PATH."
