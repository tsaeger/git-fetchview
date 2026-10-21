#!/usr/bin/env bash
# Prepare what demo.tape needs: a scratch clone of a Linux kernel repository
# (sharing its objects, so it costs almost nothing and your repo is never
# written to), a release build, and an env file the tape sources.
#
#   docs/demo/setup.sh [path/to/linux]     # default: ~/dev/linux
#   nix shell nixpkgs#ttyd --command vhs docs/demo/demo.tape
#
# The GIF is written to docs/demo/demo.gif, which Git LFS tracks.
set -euo pipefail

kernel=${1:-$HOME/dev/linux}
work=${DEMO_DIR:-/tmp/fv-demo}
repo=$(cd "$(dirname "$0")/../.." && pwd)

cargo build --release --manifest-path "$repo/Cargo.toml"

rm -rf "$work"
mkdir -p "$work"
git clone -q --shared --no-checkout "$kernel" "$work/linux"
git -C "$work/linux" remote set-url origin \
    git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
git -C "$work/linux" restore --source=HEAD --worktree MAINTAINERS

cat > "$work/gitconfig" <<'CONF'
[fetchview]
	theme = kanagawa
CONF

cat > "$work/env" <<ENV
export PS1='\\[\\e[38;2;126;156;216m\\]linux\\[\\e[0m\\] \\\$ '
export GIT_CONFIG_GLOBAL="$work/gitconfig" TIGRC_USER=/dev/null
export PATH="$repo/target/release:\$PATH"
cd "$work/linux"
ENV

echo "ready: $work (now run vhs on docs/demo/demo.tape from the repo root)"
