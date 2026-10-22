# Collecting from merge commits. Run docs/demo/setup.sh first, then, from the
# repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/merges.tape
# Reads only the local history, so it needs no network.
Output docs/demo/merges.gif
Source docs/demo/style.tape
Source docs/demo/spaced.tape

Type "# Linus pulls from other trees. Collect the ones merged in 6.12:"
Enter
Sleep 600ms
Type "git fetchview add merges v6.11..v6.12"
Enter
Sleep 3s

# Space marks the row under the cursor.
Type "jj"
Sleep 500ms
Type " "
Sleep 500ms
Type "j"
Type " "
Sleep 2s

# Search, then a marks every row that matches.
Type "i"
Type "kernel/git/"
Sleep 500ms
Escape
Sleep 1500ms
Type "a"
Sleep 3s
Enter
Sleep 2500ms

Type "git fetchview list | cut -f1,2 | column -t | head -n 8"
Enter
Sleep 4s
