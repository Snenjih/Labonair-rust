# labonair-shell-integration (bashrc)
# Emits OSC 7 (cwd) and OSC 133 command-boundary markers.

if [ -z "$__LABONAIR_HOOKS_LOADED" ]; then
  __LABONAIR_HOOKS_LOADED=1
  [ -f /etc/profile ] && source /etc/profile
  [ -f /etc/bashrc ] && source /etc/bashrc
  if [ -f "$HOME/.bash_profile" ]; then source "$HOME/.bash_profile"
  elif [ -f "$HOME/.bash_login" ]; then source "$HOME/.bash_login"
  elif [ -f "$HOME/.profile" ]; then source "$HOME/.profile"; fi
  [ -f "$HOME/.bashrc" ] && source "$HOME/.bashrc"

  _labonair_urlencode() {
    local LC_ALL=C s="$1" i c
    for (( i=0; i<${#s}; i++ )); do
      c="${s:i:1}"
      case "$c" in
        [a-zA-Z0-9/._~-]) printf '%s' "$c" ;;
        *) printf '%%%02X' "'$c" ;;
      esac
    done
  }

  _labonair_precmd() {
    local _labonair_ret=$?
    printf '\e]133;D;%s\e\\' "$_labonair_ret"
    printf '\e]7;file://%s%s\e\\' "${HOSTNAME:-$(uname -n 2>/dev/null)}" "$(_labonair_urlencode "$PWD")"
    printf '\e]0;\a'
    if [ -n "$LABONAIR_BLOCKS" ]; then
      if [ -n "$_labonair_block_seen" ]; then PS1='\n\n\[\e]133;B\e\\\]'
      else PS1='\n\[\e]133;B\e\\\]'; fi
    elif [ -z "$__LABONAIR_PS1_INJECTED" ]; then
      PS1='\[\e]133;B\e\\\]'"$PS1"
      __LABONAIR_PS1_INJECTED=1
    fi
    printf '\e]133;A\e\\'
  }

  _labonair_preexec() {
    local cmd="${1//[[:cntrl:]]/ }"
    _labonair_block_seen=1
    printf '\e]133;C;%s\e\\' "${cmd:0:256}"
  }

  if [ "${__bp_imported:-}" = "defined" ]; then
    precmd_functions+=(_labonair_precmd)
    [ -n "$LABONAIR_BLOCKS" ] && preexec_functions+=(_labonair_preexec)
  else
    case ":${PROMPT_COMMAND:-}:" in
      *":_labonair_precmd:"*) ;;
      *) PROMPT_COMMAND="_labonair_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
    esac
    if [ -z "$LABONAIR_BLOCKS" ]; then
      PS0='\[\e]133;C\e\\\]'"${PS0:-}"
    else
      _labonair_arm_preexec() { _labonair_preexec_armed=1; }
      PROMPT_COMMAND="${PROMPT_COMMAND}${PROMPT_COMMAND:+;}_labonair_arm_preexec"
      _labonair_preexec_trap() {
        [ -z "${_labonair_preexec_armed:-}" ] && return
        [ -n "${COMP_LINE:-}" ] && return
        _labonair_preexec_armed=
        local cmd
        cmd="$(HISTTIMEFORMAT= builtin history 1 2>/dev/null | sed -E 's/^[[:space:]]*[0-9]+[[:space:]]*//')"
        [ -z "$cmd" ] && cmd="$BASH_COMMAND"
        _labonair_preexec "$cmd"
      }
      trap '_labonair_preexec_trap' DEBUG
    fi
  fi
  _labonair_precmd
fi
:
