# Themes. Run docs/demo/setup.sh first, then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/themes.tape
# Reads only the local history, so it needs no network.
Output docs/demo/themes.gif
Source docs/demo/style.tape
Set TypingSpeed 35ms

Hide
Type "git fetchview add --all merges v6.11..v6.12 2>/dev/null && clear"
Enter
Sleep 3s
Show

Type "# The same picker in each theme. Set one with: git config fetchview.theme nord"
Enter
Sleep 1s

Type "git fetchview --theme kanagawa"
Enter
Sleep 1500ms
Type "jjj"
Type "i"
Type "net"
Sleep 400ms
Escape
Sleep 2500ms
Type "q"
Sleep 500ms

Type "clear"
Enter
Type "git fetchview --theme gruvbox"
Enter
Sleep 1500ms
Type "jjj"
Type "i"
Type "net"
Sleep 400ms
Escape
Sleep 2500ms
Type "q"
Sleep 500ms

Type "clear"
Enter
Type "git fetchview --theme nord"
Enter
Sleep 1500ms
Type "jjj"
Type "i"
Type "net"
Sleep 400ms
Escape
Sleep 2500ms
Type "q"
Sleep 500ms

Type "clear"
Enter
Type "git fetchview --theme catppuccin"
Enter
Sleep 1500ms
Type "jjj"
Type "i"
Type "net"
Sleep 400ms
Escape
Sleep 2500ms
Type "q"
Sleep 500ms
