# The trail. Run docs/demo/setup.sh first, then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/trail.tape
# Needs network access to git.kernel.org. The URL carries a made-up password.
Output docs/demo/trail.gif
Source docs/demo/style.tape
Source docs/demo/spaced.tape

Hide
Type "git config --global --add fetchview.hideRefsFrom '' && clear"
Enter
Show

Type "# This remote's URL has a password in it:"
Enter
Type "git remote add work https://dev:s3cret@git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git"
Enter
Sleep 800ms
Type "git fetchview"
Enter
Sleep 1500ms
Type "j"
Sleep 600ms
Enter
Hide
Wait+Screen@120s /refs of work/
Show
Sleep 1500ms
Type "j"
Sleep 700ms
Enter

# tig opens once the fetch is done. Quit it to get back to the trail.
Hide
Wait+Screen@60s /\[main\]/
Show
Sleep 2s
Type "q"
Sleep 5s

# The trail is plain git, so it works without this tool.
Type "clear"
Enter
Type "# The trail has no password and needs no git-fetchview. Same fetch, by hand:"
Enter
Set TypingSpeed 15ms
Type "git fetch https://dev@git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git refs/heads/main"
Enter
Sleep 6s
Type "git log -1 --oneline FETCH_HEAD"
Enter
Sleep 4s
