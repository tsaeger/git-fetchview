# Searching, filtering and the help screen in the remote list. Run
# docs/demo/setup.sh first, then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/search.tape
# Reads only the local history, so it needs no network.
Output docs/demo/search.gif
Source docs/demo/style.tape
Source docs/demo/spaced.tape
Set TypingSpeed 45ms

Hide
Type "git fetchview add --all merges v6.11..v6.12 2>/dev/null && clear"
Enter
Sleep 3s
Show

Type "# Search takes fzf-style terms: space is and, ! is not, | is or."
Enter
Type "git fetchview"
Enter
Sleep 2s
Type "i"
Type "net"
Sleep 1500ms
Type " !next"
Sleep 2500ms
Type " | bpf"
Sleep 2500ms
Escape
Sleep 1500ms

# Ctrl-r switches the box to a regular expression.
Type "c"
Sleep 800ms
Type "i"
Ctrl+R
Type "^(net|bpf)/"
Sleep 3s
Escape
Sleep 1500ms
# Back to plain terms.
Type "c"
Type "i"
Ctrl+R
Escape
Sleep 800ms

# f filters just the column under the cursor; l moves the cursor.
Type "c"
Sleep 800ms
Type "l"
Type "f"
Type "github"
Sleep 1500ms
Escape
Sleep 2500ms
Type "c"
Sleep 1500ms

# v hides the info panel, ? lists every key.
Type "jj"
Sleep 1500ms
Type "v"
Sleep 2s
Type "v"
Sleep 1s
Type "?"
Sleep 5s
Type "x"
Sleep 500ms
Type "q"
Sleep 500ms
