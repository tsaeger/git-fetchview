# Shared look for every demo, so they read as one set. Each tape starts with
#   Output docs/demo/<name>.gif
#   Source docs/demo/style.tape
# The palette is Kanagawa, the default git-fetchview theme. Keep sizes here, not
# in the individual tapes, so changing the look is a one-file edit.
Set Shell "bash"
Set FontSize 17
Set Width 1180
Set Height 660
Set Padding 16
Set TypingSpeed 55ms
Set Framerate 20
Set Theme { "name": "Kanagawa", "black": "#090618", "red": "#C34043", "green": "#76946A", "yellow": "#C0A36E", "blue": "#7E9CD8", "magenta": "#957FB8", "cyan": "#6A9589", "white": "#C8C093", "brightBlack": "#727169", "brightRed": "#E82424", "brightGreen": "#98BB6C", "brightYellow": "#E6C384", "brightBlue": "#7FB4CA", "brightMagenta": "#938AA9", "brightCyan": "#7AA89F", "brightWhite": "#DCD7BA", "background": "#1F1F28", "foreground": "#DCD7BA", "selection": "#2D4F67", "cursor": "#C8C093" }

# Start in the scratch kernel clone made by docs/demo/setup.sh, with a short
# prompt and a clean screen.
Hide
Type "source /tmp/fv-demo/env && clear"
Enter
Sleep 500ms
Show
