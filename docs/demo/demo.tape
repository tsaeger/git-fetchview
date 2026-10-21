# The README teaser. Run docs/demo/setup.sh before EVERY recording (it resets the
# scratch clone; otherwise trees collected by the last run show as (have)).
# Then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/demo.tape
# The GIF is tracked with Git LFS (see .gitattributes). It needs network access
# to git.kernel.org for the fetch.
Output docs/demo/demo.gif
Source docs/demo/style.tape

# 1. Collect remotes from MAINTAINERS.
Type "# MAINTAINERS lists hundreds of kernel trees. Collect the networking ones:"
Enter
Sleep 800ms
Type "git fetchview add maintainers"
Enter
Sleep 3s
Type "/netdev"
Sleep 800ms
Enter
Sleep 2500ms
Type "a"
Sleep 3s
Enter
Sleep 2500ms

# 2. Pick a remote. The refs list leaves out the tags mainline and stable
# already have. The first open fetches those lists, so the wait is cut from the
# recording and the list appears once it is ready.
Type "clear"
Enter
Type "# Now pick one. Tags mainline and stable already have are hidden:"
Enter
Sleep 600ms
Type "git fetchview"
Enter
Sleep 2000ms
Type "i"
Type "net-next"
Sleep 600ms
Escape
Sleep 1000ms
Hide
Type "l"
Wait+Screen@120s /hidden/
Show
Sleep 3s

# 3. Pick main.
Type "j"
Sleep 700ms
Enter
Sleep 800ms

# 4. Browse FETCH_HEAD in tig. Wait for it instead of guessing how long the
# fetch takes, so no keys are typed early.
Wait+Screen@60s /\[main\]/
Sleep 1500ms
Type "jjj"
Sleep 1500ms
Type "q"
Sleep 3500ms
