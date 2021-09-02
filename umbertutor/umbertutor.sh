wait_for_action() {
  while true
  do
    last_match=$(cat action_log | grep "$1" | tail -1)
    if [ -n "$last_match" ]
    then
      echo $last_match
      return 0
    fi
    sleep 1
  done
}
wait_for_action_check_window() {
  id=$(wait_for_action "$1" | cut -d\  -f2)
  if xprop -id $id WM_CLASS | grep -i "$2" >/dev/null
  then
    return 0
  fi
  wait_for_action_check_window "$@"
}
step() {
echo > action_log
echo "\r[x] "
echo -n "[ ] $@" 
}
tput civis
echo -n "[ ] welcome to umbertutor"
step "to start, let's open a terminal (alt + shift + enter)"
wait_for_action_check_window SetupWindow Alacritty
step "let's run the command launcher (alt + r)"
wait_for_action_check_window SetupWindow rofi
step "input 'xeyes' in the command launcher to start xeyes app"
wait_for_action_check_window SetupWindow xeyes
step "change focus of windows (alt + space)"
wait_for_action >/dev/null SwitchWindow
step "change layout to monocle (alt + f)"
wait_for_action >/dev/null "ChangeLayout monocle" 
step "change layout to horizontal (alt + f)"
wait_for_action >/dev/null "ChangeLayout bsph"
step "toggle the window gap (alt + g)"
wait_for_action >/dev/null ToggleGap
step "move to workspace b (alt + b)"
wait_for_action >/dev/null ChangeWorkspace
step "open a terminal (alt + shift + enter)"
wait_for_action_check_window SetupWindow Alacritty
step "well done ! you can quit umberwm (ctrl + alt + q)"
while true
do
  sleep 1
done

