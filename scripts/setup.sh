#!/bin/sh
# 
# Copyright (c) 2025 murilo ijanc' <murilo@ijanc.org>
#
# Permission to use, copy, modify, and distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
# 
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
# ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
# ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
# OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
#
set -eu

# Defaults
DRY_RUN=0
INSTALL_K3D=1
INSTALL_DOCKER=1
INSTALL_KUBECTL=1
VERBOSE=0

log() {
    printf '[INFO] %s\n' "$*"
}

error() {
    printf '[ERROR] %s\n' "$*" >&2
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

run_cmd() {
    if [ "$DRY_RUN" -eq 1 ]; then
        log "[dry-run] $*"
    else
        sh -c "$*"
    fi
}

# Enable verbose (debug) mode if requested
enable_verbose() {
    if [ "$VERBOSE" -eq 1 ]; then
        set -x
    fi
}

# Disable verbose on failures for better logs
trap 'set +x >/dev/null 2>&1 || true' EXIT

collect_version() {
    CMD="$1"
    if command_exists "$CMD"; then
        case "$CMD" in
            kubectl)
                VERSION="$($CMD version --client 2>/dev/null | head -n1 || true)"
                ;;
            docker)
                VERSION="$($CMD --version 2>/dev/null | head -n1 || true)"
                ;;
            k3d)
                VERSION="$($CMD version 2>/dev/null | head -n1 || true)"
                ;;
            *)
                VERSION="$($CMD --version 2>/dev/null | head -n1 || true)"
                ;;
        esac
        log $(printf "%s: %s\n" "$CMD" "${VERSION:-unknown}")
    else
        log $(printf "%s: not installed\n" "$CMD")
    fi
}

# Detect sudo (only if necessary)
if [ "$(id -u)" -eq 0 ]; then
    SUDO=""
else
    SUDO="sudo"
fi

install_k3d() {
    if [ "$INSTALL_K3D" -eq 0 ]; then
        log "Skipping k3d"
        return
    fi

    if command_exists k3d; then
        log "k3d already installed"
        return
    fi

    log "Installing k3d..."
    run_cmd "curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | sh"
}

install_docker_apt() {
    if command_exists docker; then
        log "docker already installed"
    else
        log "Installing docker..."
        run_cmd "$SUDO apt-get install -y docker.io"
    fi
}

install_kubectl_apt() {
    if command_exists kubectl; then
        log "kubectl already installed"
    else
        log "Installing kubectl..."
        run_cmd "$SUDO apt-get install -y kubectl || true"
    fi
}

########## macOS ##########

install_macos() {
    if ! command_exists brew; then
        error "Homebrew required: https://brew.sh/"
        exit 1
    fi

    [ "$INSTALL_DOCKER" -eq 1 ] && \
        (command_exists docker || run_cmd "brew install docker")

    [ "$INSTALL_KUBECTL" -eq 1 ] && \
        (command_exists kubectl || run_cmd "brew install kubernetes-cli")

    install_k3d
}

########## Linux ##########

install_linux() {
    . /etc/os-release

    case "$ID" in
        ubuntu|debian)
            log "Updating apt..."
            run_cmd "$SUDO apt-get update"
            [ "$INSTALL_DOCKER" -eq 1 ] && install_docker_apt
            [ "$INSTALL_KUBECTL" -eq 1 ] && install_kubectl_apt
            install_k3d
            ;;
        alpine)
            [ "$INSTALL_DOCKER" -eq 1 ] && \
                (command_exists docker || run_cmd "$SUDO apk add --no-cache docker")

            [ "$INSTALL_KUBECTL" -eq 1 ] && \
                (command_exists kubectl || run_cmd "$SUDO apk add --no-cache kubectl")

            install_k3d
            ;;
        arch|artix)
            log "Updating pacman..."
            run_cmd "$SUDO pacman -Sy --noconfirm"

            [ "$INSTALL_DOCKER" -eq 1 ] && \
                (command_exists docker || run_cmd "$SUDO pacman -S --noconfirm docker")

            [ "$INSTALL_KUBECTL" -eq 1 ] && \
                (command_exists kubectl || run_cmd "$SUDO pacman -S --noconfirm kubectl")

            install_k3d
            ;;
        *)
            error "Unsupported Linux distro: $ID"
            exit 1
            ;;
    esac
}

########## CLI ARG PARSE ##########

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY_RUN=1 ;;
        --no-k3d) INSTALL_K3D=0 ;;
        --no-docker) INSTALL_DOCKER=0 ;;
        --no-kubectl) INSTALL_KUBECTL=0 ;;
        --verbose) VERBOSE=1 ;;
        *)
            error "Unknown option: $1"
            exit 1 ;;
    esac
    shift
done

enable_verbose

########## Main ##########

OS="$(uname -s)"
log "Detected OS: $OS"

case "$OS" in
    Darwin) install_macos ;;
    Linux)  install_linux ;;
    *)
        error "Unsupported OS: $OS"
        exit 1 ;;
esac

set +x >/dev/null 2>&1 || true
echo
log "========== INSTALLATION SUMMARY =========="
collect_version docker
collect_version kubectl
collect_version k3d
log "=========================================="
log "Done."

