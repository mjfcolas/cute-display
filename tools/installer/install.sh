#!/bin/sh
# Installs or updates Cute Display's installer, then runs `cute-display setup`, or the
# command it is given:
#
#   curl -LsSf https://github.com/mjfcolas/cute-display/releases/latest/download/install.sh | sh
#
# `just release` fills in the version.
set -eu

WHEEL="https://github.com/mjfcolas/cute-display/releases/download/v@VERSION@/cute_display_installer-@VERSION@-py3-none-any.whl"

# Run only once read whole: piped into sh, a download cut short would run half a script.
main() {
    if ! command -v uv >/dev/null 2>&1; then
        echo 'Installing uv, which runs the installer...'
        curl -LsSf https://astral.sh/uv/install.sh | sh
        PATH="${UV_INSTALL_DIR:-${XDG_BIN_HOME:-$HOME/.local/bin}}:$PATH"
        if ! command -v uv >/dev/null 2>&1; then
            echo 'uv could not be installed: see https://docs.astral.sh/uv/getting-started/installation/' >&2
            exit 1
        fi
    fi

    uv tool install --force --quiet "$WHEEL"
    uv tool update-shell >/dev/null 2>&1 || true
    installer="$(uv tool dir --bin)/cute-display"

    if [ $# -eq 0 ]; then
        set -- setup
    fi

    # Piped into sh, this script's input is the download: the setup's screens need the keyboard.
    if [ ! -t 0 ] && (exec </dev/tty) 2>/dev/null; then
        exec "$installer" "$@" </dev/tty
    fi
    exec "$installer" "$@"
}

main "$@"
