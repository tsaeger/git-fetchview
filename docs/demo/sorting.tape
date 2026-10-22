# Sorting. Run docs/demo/setup.sh first, then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/sorting.tape
# Needs network access to git.kernel.org for the ref list.
Output docs/demo/sorting.gif
Source docs/demo/style.tape

Type "# Git lists tags as text, so v6.10 lands before v6.2. Sorting fixes that:"
Enter
Sleep 600ms
Type "git fetchview"
Enter
Sleep 1500ms
Enter
Hide
Wait+Screen@120s /refs\/tags/
Show
Sleep 1500ms

# Narrow to the 6.x release tags.
Type "i"
Type "tags/v6."
Sleep 600ms
Escape
Sleep 3500ms

# Sort mode: h and l move the sort column, the header shows an arrow.
Type "s"
Sleep 1500ms
Type "l"
Sleep 3500ms

# r reverses it, so the newest release is on top.
Type "r"
Sleep 3500ms
Type "q"
Sleep 1s
