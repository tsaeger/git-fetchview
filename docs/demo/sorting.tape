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

# s sorts the column under the column cursor: ascending, with v6.2 before v6.10.
Type "s"
Sleep 3500ms

# A second press runs it descending, so the newest release is on top.
Type "s"
Sleep 3500ms

# A third turns the sort off and restores git's order.
Type "s"
Sleep 3s
Type "q"
Sleep 1s
