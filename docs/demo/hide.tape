# Hiding refs other trees carry. Run docs/demo/setup.sh first (it clears the
# cache), then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/hide.tape
# Needs network access to git.kernel.org. The first look fetches the tag lists
# of mainline and stable, so that wait is cut from the recording.
Output docs/demo/hide.gif
Source docs/demo/style.tape

Type "K=https://git.kernel.org/pub/scm/linux/kernel/git"
Enter
Type "git fetchview add --all url $K/netdev/net-next.git"
Enter
Sleep 1500ms
Type "clear"
Enter

Type "# net-next carries every mainline tag. Those are hidden, so what is left is net-next's own:"
Enter
Sleep 800ms
Type "git fetchview --refresh"
Enter
Sleep 1500ms
Type "j"
Sleep 800ms
Hide
Enter
Wait+Screen@180s /hidden/
Show
Sleep 3500ms

# Mainline itself is never filtered.
Escape
Sleep 800ms
Type "k"
Sleep 800ms
Hide
Enter
Wait+Screen@120s /refs of origin/
Show
Sleep 3500ms
Type "q"
Sleep 800ms
