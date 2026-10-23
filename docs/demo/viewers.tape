# Viewers. Run docs/demo/setup.sh first, then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/viewers.tape
# Needs network access to git.kernel.org. Waits are hidden so keys are never
# typed before a screen is up.
Output docs/demo/viewers.gif
Source docs/demo/style.tape
Source docs/demo/spaced.tape

Hide
Type "git config --global --add fetchview.hideRefsFrom ''"
Enter
Type "git remote remove origin && git remote add net https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git && clear"
Enter
Show

Type "# The same fetch, browsed four ways. First tig, the default:"
Enter
Sleep 600ms
Type "git fetchview"
Enter
Sleep 1s
Enter
Hide
Wait+Screen@120s /refs of net/
Show
Sleep 600ms
Type "i"
Type "heads/main"
Sleep 400ms
Escape
Sleep 500ms
Enter
Hide
Wait+Screen@60s /\[main\]/
Show
Sleep 2500ms
Type "q"
Sleep 800ms

Type "clear"
Enter
Type "# git log, through your pager:"
Enter
Type "git fetchview --viewer git-log"
Enter
Sleep 1s
Enter
Hide
Wait+Screen@120s /refs of net/
Show
Sleep 600ms
Type "i"
Type "heads/main"
Sleep 400ms
Escape
Sleep 500ms
Enter
Hide
Wait+Screen@60s /commit /
Show
Sleep 3s
Type "q"
Sleep 800ms

Type "clear"
Enter
Type "# fzf, with a preview of each commit:"
Enter
Type "git fetchview --viewer fzf"
Enter
Sleep 1s
Enter
Hide
Wait+Screen@120s /refs of net/
Show
Sleep 600ms
Type "i"
Type "heads/main"
Sleep 400ms
Escape
Sleep 500ms
Enter
Hide
Wait+Screen@60s /\d+\/\d+/
Show
Sleep 1s
Type "bpf"
Sleep 3s
Escape
Sleep 800ms

Type "clear"
Enter
Type "# Or any command. {} stands for FETCH_HEAD:"
Enter
Type "git fetchview --viewer 'git --no-pager log -1 --stat --oneline {}'"
Enter
Sleep 1s
Enter
Hide
Wait+Screen@120s /refs of net/
Show
Sleep 600ms
Type "i"
Type "heads/main"
Sleep 400ms
Escape
Sleep 500ms
Enter
Hide
Wait+Screen@60s /changed/
Show
Sleep 4s
