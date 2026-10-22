# Collecting by hand and managing the list. Run docs/demo/setup.sh first, then,
# from the repo root:
#   nix shell nixpkgs#ttyd --command vhs docs/demo/collect.tape
# Nothing here touches the network. HOME is moved so the shared file lands in
# the scratch directory, not in your real home.
Output docs/demo/collect.gif
Source docs/demo/style.tape
Source docs/demo/spaced.tape
Set TypingSpeed 30ms
Set Height 760

Hide
Type "export HOME=/tmp/fv-demo/home && mkdir -p $HOME && clear"
Enter
Show

Type "# Add a URL, previewing it first with --dry-run:"
Enter
Type "K=https://git.kernel.org/pub/scm/linux/kernel/git"
Enter
Type "git fetchview add --all --dry-run url $K/netdev/net.git"
Enter
Sleep 2500ms
Type "git fetchview add --all url $K/netdev/net.git"
Enter
Sleep 2s

Type "# Or many at once, from a file or stdin:"
Enter
Type "printf '%s\n' $K/bpf/bpf-next.git $K/rdma/rdma.git | git fetchview add --all url --from-file -"
Enter
Sleep 2500ms

Type "git fetchview list | cut -f1,2 | column -t"
Enter
Sleep 3500ms

Type "# Drop one by label:"
Enter
Type "git fetchview remove rdma/rdma"
Enter
Sleep 2s
Type "git fetchview list | cut -f1 | column -t"
Enter
Sleep 3s

Type "clear"
Enter
Type "# Keep a list shared by every clone, in a file of your choice:"
Enter
Type "git fetchview add --all --global --file ~/kernel.remotes url $K/tip/tip.git"
Enter
Sleep 3s
Type "git config --global --get-all fetchview.remotes"
Enter
Sleep 4s
