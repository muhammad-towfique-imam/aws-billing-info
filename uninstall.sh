#!/bin/sh
set -euo pipefail

REPO="muhammad-towfique-imam/aws-billing-info"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="aws-billing-info"
RC_FILES="~/.bashrc ~/.zshrc ~/.profile ~/.config/fish/config.fish"
PATH_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""

if [ ! -f "$INSTALL_DIR/$BIN_NAME" ]; then
  echo "$BIN_NAME not found in $INSTALL_DIR" >&2
else
  rm -f "$INSTALL_DIR/$BIN_NAME"
  echo "Removed $BIN_NAME from $INSTALL_DIR"
fi

if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
  for rc in $RC_FILES; do
    expanded_rc=$(eval echo "$rc")
    [ -f "$expanded_rc" ] || continue
    case ":$PATH:" in
      *":$INSTALL_DIR:"*)
        sed -i "/$PATH_LINE/d" "$expanded_rc" 2>/dev/null || true
        echo "Removed $PATH_LINE from $expanded_rc"
        ;;
    esac
  done
fi

echo "Uninstall complete. Restart your shell or run 'hash -r' if $BIN_NAME is still in your PATH."
