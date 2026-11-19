#!/bin/sh
#
#  This file is part of the OmniDiff code diffing tool.
#
#  Copyright (C) 2026 Marko Ivankovic
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU Affero General Public License as published
#  by the Free Software Foundation, version 3 of the License.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program. If not, see <https://www.gnu.org/licenses/>.

# Installs omnidiff the way that suits the system it runs on:
#
#     curl -fsSL https://ivankovic.github.io/omnidiff/install.sh | sh
#     curl -fsSL https://ivankovic.github.io/omnidiff/install.sh | sh -s -- --method tarball
#
# It reuses what the project already publishes instead of being one more way to install:
#
#   brew     where Homebrew is on PATH: the tap's formula, so brew owns upgrades and removal.
#   apt      on Debian and Ubuntu: the signed apt repository, so `apt upgrade` brings new
#            versions. This puts a signing key and a sources entry on the machine and needs root,
#            so the script asks first, and where it cannot ask (no terminal) it does not do it.
#            Declining, or having no terminal, installs the tarball instead.
#   tarball  everywhere else, and wherever asked for: the release's prebuilt binary - on Linux
#            the static musl build, which runs on any distribution - checked against the
#            release's SHA256SUMS.txt and unpacked under a prefix (~/.local) that needs no root,
#            with the man page and the shell completions beside it.
#
# It never builds from source: where no prebuilt binary fits, it says so and names the document
# that lists the other ways. NixOS gets the `nix` command to run rather than a change to the
# system.
#
# POSIX sh, not bash: Debian's sh is dash and Alpine's is busybox. The whole script is the body of
# `main`, called on the last line, so a download cut short cannot run half of it.
# scripts/install_e2e.py runs it in containers of several distributions.

set -eu

REPOSITORY="ivankovic/omnidiff"
# Overridable so the end-to-end test can point the script at a local copy of a release.
RELEASES_URL="${OMNIDIFF_RELEASES_URL:-https://github.com/$REPOSITORY/releases}"
APT_URL="${OMNIDIFF_APT_URL:-https://ivankovic.github.io/omnidiff/apt}"
DOCS_URL="https://github.com/$REPOSITORY/blob/main/docs/README.md#installation"
KEYRING="/etc/apt/keyrings/omnidiff-archive-keyring.gpg"
SOURCES="/etc/apt/sources.list.d/omnidiff.list"

say() {
    printf '%s\n' "$*"
}

warn() {
    printf 'omnidiff install: %s\n' "$*" >&2
}

die() {
    warn "$*"
    exit 1
}

have() {
    command -v "$1" >/dev/null 2>&1
}

usage() {
    cat <<EOF
Installs omnidiff: through Homebrew or apt where the system has them, else from the release's
prebuilt binary.

Usage: install.sh [--method auto|brew|apt|tarball] [--prefix DIR] [--version X.Y.Z] [--yes]

  --method   auto (the default) picks brew where it is on PATH, apt on Debian and Ubuntu, and
             the tarball otherwise. apt adds a signing key and a sources entry as root: auto asks
             first, and installs the tarball instead if the answer is no or nobody can be asked.
  --prefix   where the tarball goes: DIR/bin, DIR/share/man, DIR/share/doc (default: ~/.local)
  --version  the release the tarball is taken from (default: the latest)
  --yes      answer yes to the apt question
EOF
}

# Whether somebody is there to answer a question. The script itself usually arrives on stdin
# (`curl ... | sh`), so the question is put to the terminal, not to stdin.
can_ask() {
    [ -r /dev/tty ] && [ -w /dev/tty ] && (: </dev/tty) 2>/dev/null
}

# Asks a yes/no question on the terminal; no is the default.
confirm() {
    printf '%s [y/N] ' "$1" >/dev/tty
    read -r answer </dev/tty || answer=""
    case "$answer" in
        y | Y | yes | Yes | YES) return 0 ;;
        *) return 1 ;;
    esac
}

download() {
    # download URL FILE
    if have curl; then
        curl -fsSL --proto '=https' --tlsv1.2 -o "$2" "$1"
    elif have wget; then
        wget -q -O "$2" "$1"
    else
        die "needs curl or wget to download $1"
    fi
}

sha256_of() {
    if have sha256sum; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif have shasum; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        die "needs sha256sum or shasum to check the download"
    fi
}

# The release asset for this machine, or nothing if no prebuilt binary fits it.
tarball_target() {
    case "$(uname -m)" in
        x86_64 | amd64) cpu="x86_64" ;;
        aarch64 | arm64) cpu="aarch64" ;;
        *) return 0 ;;
    esac
    case "$(uname -s)" in
        Linux) say "$cpu-unknown-linux-musl" ;;
        Darwin) say "$cpu-apple-darwin" ;;
        *) ;;
    esac
}

# Runs a command as root: directly when already root, else through sudo.
as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        sudo "$@"
    fi
}

is_debian_like() {
    [ -f /etc/debian_version ] && have apt-get && have dpkg
}

# Whether the apt repository has a package for this machine and the script can become root.
apt_possible() {
    is_debian_like || return 1
    case "$(dpkg --print-architecture)" in
        amd64 | arm64) ;;
        *) return 1 ;;
    esac
    [ "$(id -u)" -eq 0 ] || have sudo
}

install_brew() {
    say "Installing with Homebrew: brew install $REPOSITORY/omnidiff"
    brew install "$REPOSITORY/omnidiff"
    say ""
    say "Installed. Upgrade with 'brew upgrade omnidiff'; remove with 'brew uninstall omnidiff'."
}

install_apt() {
    say "Installing from the OmniDiff apt repository ($APT_URL):"
    say "  its signing key  -> $KEYRING"
    say "  a sources entry  -> $SOURCES"
    say "  then: apt-get update && apt-get install omnidiff"
    key="$WORK/omnidiff-archive-keyring.gpg"
    download "$APT_URL/omnidiff-archive-keyring.gpg" "$key"
    [ -s "$key" ] || die "the apt repository's signing key came back empty"
    printf 'deb [signed-by=%s] %s stable main\n' "$KEYRING" "$APT_URL" >"$WORK/omnidiff.list"
    as_root install -d -m 0755 /etc/apt/keyrings
    as_root install -m 0644 "$key" "$KEYRING"
    as_root install -m 0644 "$WORK/omnidiff.list" "$SOURCES"
    as_root apt-get update
    as_root apt-get install -y omnidiff
    say ""
    say "Installed. New versions arrive with 'apt upgrade'."
    say "Remove with: sudo apt-get remove omnidiff && sudo rm $SOURCES $KEYRING"
}

install_tarball() {
    target="$(tarball_target)"
    if [ -z "$target" ]; then
        die "there is no prebuilt omnidiff for $(uname -s) on $(uname -m). Other ways to install: $DOCS_URL"
    fi
    if [ -n "$VERSION" ]; then
        base="$RELEASES_URL/download/v$VERSION"
    else
        base="$RELEASES_URL/latest/download"
    fi
    asset="omnidiff-$target.tar.gz"
    extras="omnidiff-completions-and-man.tar.gz"
    say "Installing the prebuilt binary ($asset) under $PREFIX"

    download "$base/SHA256SUMS.txt" "$WORK/SHA256SUMS.txt"
    for file in "$asset" "$extras"; do
        download "$base/$file" "$WORK/$file"
        # The line of SHA256SUMS.txt that names exactly this file ("<hash>  <name>", the name
        # sometimes with a leading `*` or `./`).
        expected="$(awk -v name="$file" '{ n = $2; sub(/^\*/, "", n); sub(/^\.\//, "", n); if (n == name) print $1 }' "$WORK/SHA256SUMS.txt")"
        [ -n "$expected" ] || die "the release's SHA256SUMS.txt does not list $file"
        actual="$(sha256_of "$WORK/$file")"
        if [ "$expected" != "$actual" ]; then
            die "$file does not match the release's checksum (expected $expected, got $actual); nothing was installed"
        fi
    done

    mkdir -p "$WORK/binary" "$WORK/extras"
    tar -xzf "$WORK/$asset" -C "$WORK/binary"
    tar -xzf "$WORK/$extras" -C "$WORK/extras"
    [ -f "$WORK/binary/omnidiff" ] || die "$asset holds no omnidiff binary"

    bin="$PREFIX/bin"
    share="$PREFIX/share"
    mkdir -p "$bin" "$share/man/man1" "$share/doc/omnidiff" \
        "$share/bash-completion/completions" "$share/zsh/site-functions" \
        "$share/fish/vendor_completions.d"
    # Written beside the target and renamed, so a running omnidiff is replaced, not overwritten.
    cp "$WORK/binary/omnidiff" "$bin/.omnidiff.new"
    chmod 0755 "$bin/.omnidiff.new"
    mv -f "$bin/.omnidiff.new" "$bin/omnidiff"
    for doc in LICENSE THIRD-PARTY-NOTICES.md README.md; do
        if [ -f "$WORK/binary/$doc" ]; then
            cp "$WORK/binary/$doc" "$share/doc/omnidiff/$doc"
        fi
    done
    cp "$WORK/extras/omnidiff.1" "$share/man/man1/omnidiff.1"
    cp "$WORK/extras/completions/omnidiff.bash" "$share/bash-completion/completions/omnidiff"
    cp "$WORK/extras/completions/_omnidiff" "$share/zsh/site-functions/_omnidiff"
    cp "$WORK/extras/completions/omnidiff.fish" "$share/fish/vendor_completions.d/omnidiff.fish"

    say ""
    say "Installed $("$bin/omnidiff" --version) to $bin/omnidiff"
    case ":$PATH:" in
        *":$bin:"*) ;;
        *) say "$bin is not on your PATH: add it, for example with  export PATH=\"$bin:\$PATH\"" ;;
    esac
    say "Run this script again to upgrade. Remove with:"
    say "  rm $bin/omnidiff $share/man/man1/omnidiff.1 $share/bash-completion/completions/omnidiff \\"
    say "     $share/zsh/site-functions/_omnidiff $share/fish/vendor_completions.d/omnidiff.fish"
    say "  rm -r $share/doc/omnidiff"
}

# What `--method auto` comes to on this machine: brew, apt or tarball.
choose_method() {
    if have brew; then
        say "brew"
        return 0
    fi
    if apt_possible; then
        if [ "$ASSUME_YES" = "yes" ]; then
            say "apt"
            return 0
        fi
        if can_ask; then
            {
                say "This is a Debian or Ubuntu system. OmniDiff has a signed apt repository, which"
                say "brings new versions with 'apt upgrade'. Using it puts the repository's signing"
                say "key in $KEYRING and a sources entry in"
                say "$SOURCES, as root."
                say "Answering no installs the prebuilt binary under $PREFIX instead, without root."
            } >/dev/tty
            if confirm "Add the OmniDiff apt repository and its signing key?"; then
                say "apt"
                return 0
            fi
        else
            warn "nobody to ask about adding the apt repository's signing key, so not adding it;" \
                "installing the prebuilt binary instead (--method apt or --yes adds it)"
        fi
    fi
    say "tarball"
}

main() {
    METHOD="auto"
    PREFIX="${HOME:-}/.local"
    VERSION=""
    ASSUME_YES="no"
    while [ $# -gt 0 ]; do
        case "$1" in
            --method)
                [ $# -ge 2 ] || die "--method needs a value"
                METHOD="$2"
                shift 2
                ;;
            --method=*)
                METHOD="${1#--method=}"
                shift
                ;;
            --prefix)
                [ $# -ge 2 ] || die "--prefix needs a value"
                PREFIX="$2"
                shift 2
                ;;
            --prefix=*)
                PREFIX="${1#--prefix=}"
                shift
                ;;
            --version)
                [ $# -ge 2 ] || die "--version needs a value"
                VERSION="$2"
                shift 2
                ;;
            --version=*)
                VERSION="${1#--version=}"
                shift
                ;;
            --yes | -y)
                ASSUME_YES="yes"
                shift
                ;;
            --help | -h)
                usage
                exit 0
                ;;
            *)
                usage >&2
                die "unknown argument: $1"
                ;;
        esac
    done
    VERSION="${VERSION#v}"
    case "$METHOD" in
        auto | brew | apt | tarball) ;;
        *) die "--method is auto, brew, apt or tarball, not $METHOD" ;;
    esac
    if [ "$PREFIX" = "/.local" ]; then
        die "HOME is not set, so there is no default prefix: pass --prefix DIR"
    fi
    case "$PREFIX" in
        /*) ;;
        *) die "--prefix must be an absolute path, not '$PREFIX'" ;;
    esac

    if [ "$METHOD" = "auto" ] && [ -e /etc/NIXOS ]; then
        say "This is NixOS, where a script should not install into the system. Run omnidiff with:"
        say "  nix run github:$REPOSITORY"
        say "or add it to a profile with:"
        say "  nix profile install github:$REPOSITORY"
        say "(--method tarball installs the static binary under a prefix all the same.)"
        exit 0
    fi

    WORK="$(mktemp -d)"
    trap 'rm -rf "$WORK"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM

    if [ "$METHOD" = "auto" ]; then
        METHOD="$(choose_method)"
    fi
    case "$METHOD" in
        brew)
            have brew || die "--method brew needs Homebrew (https://brew.sh) on PATH"
            install_brew
            ;;
        apt)
            is_debian_like || die "--method apt needs a Debian or Ubuntu system"
            apt_possible || die "--method apt needs an amd64 or arm64 system and root or sudo"
            install_apt
            ;;
        tarball)
            install_tarball
            ;;
    esac
}

main "$@"
