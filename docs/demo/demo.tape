# The README demo. Run docs/demo/setup.sh before EVERY recording (it resets the
# scratch clone; otherwise trees collected by the last run show as (have)).
# Then, from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/demo.tape
# The GIF is tracked with Git LFS (see .gitattributes). It needs network access
# to git.kernel.org for the fetch.
Output docs/demo/demo.gif

Set Shell "bash"
Set FontSize 17
Set Width 1180
Set Height 660
Set Padding 16
Set TypingSpeed 55ms
Set Framerate 20
Set Theme { "name": "Kanagawa", "black": "#090618", "red": "#C34043", "green": "#76946A", "yellow": "#C0A36E", "blue": "#7E9CD8", "magenta": "#957FB8", "cyan": "#6A9589", "white": "#C8C093", "brightBlack": "#727169", "brightRed": "#E82424", "brightGreen": "#98BB6C", "brightYellow": "#E6C384", "brightBlue": "#7FB4CA", "brightMagenta": "#938AA9", "brightCyan": "#7AA89F", "brightWhite": "#DCD7BA", "background": "#1F1F28", "foreground": "#DCD7BA", "selection": "#2D4F67", "cursor": "#C8C093" }

Hide
Type "source /tmp/fv-demo/env && clear"
Enter
Sleep 500ms
Show

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

# 2. Pick a remote, then a ref.
Type "clear"
Enter
Type "# Now pick one, fetch it and browse it:"
Enter
Sleep 600ms
Type "git fetchview"
Enter
Sleep 2000ms
Type "/net-next"
Sleep 600ms
Enter
Sleep 1000ms
Type "l"
Sleep 2500ms

# 3. Sort the refs naturally, newest first, then pick main.
Type "s"
Sleep 700ms
Type "l"
Sleep 900ms
Type "r"
Sleep 2200ms
Type "s"
Sleep 500ms
Type "/main"
Sleep 500ms
Enter
Sleep 1200ms
Enter

# 4. Browse FETCH_HEAD in tig. Wait for it instead of guessing how long the
# fetch takes, so no keys are typed early.
Wait+Screen@60s /\[main\]/
Sleep 1500ms
Type "jjj"
Sleep 1500ms
Type "q"
Sleep 3500ms
