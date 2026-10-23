# Adding, browsing once, promoting and removing in the remote list. Run
# docs/demo/setup.sh first, then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/picker.tape
# Needs network access to git.kernel.org for the one-off browse.
# The list starts with only this clone's origin.
Output docs/demo/picker.gif
Source docs/demo/style.tape
Source docs/demo/spaced.tape

Type "# The picker manages its own list. n adds a URL:"
Enter
Type "git fetchview"
Enter
Sleep 2s
Type "n"
Sleep 500ms
Set TypingSpeed 20ms
Type "https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net.git"
Set TypingSpeed 55ms
Sleep 800ms
Enter
Sleep 2500ms
Type "n"
Sleep 500ms
Set TypingSpeed 20ms
Type "https://git.kernel.org/pub/scm/linux/kernel/git/netdev/net-next.git"
Set TypingSpeed 55ms
Sleep 800ms
Enter
Sleep 2500ms

# p makes the collected remote a real git remote (the dot).
Type "p"
Sleep 1500ms
Enter
Sleep 3s

# o browses a URL once, without saving it.
Type "o"
Sleep 500ms
Set TypingSpeed 20ms
Type "https://git.kernel.org/pub/scm/git/git.git"
Set TypingSpeed 55ms
Sleep 800ms
Enter
Hide
Wait+Screen@120s /refs of/
Show
Sleep 2500ms
Escape
Sleep 2s

# d removes a collected remote after a y.
Type "G"
Sleep 800ms
Type "d"
Sleep 2s
Type "y"
Sleep 3s
Type "q"
Sleep 500ms
