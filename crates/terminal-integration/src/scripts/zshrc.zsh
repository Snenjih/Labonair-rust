# labonair-shell-integration (zshrc)
# Emits OSC 7 (cwd) and OSC 133 command-boundary markers.

{
  _labonair_user_zdotdir="${LABONAIR_USER_ZDOTDIR:-$HOME}"
  [ -f "$_labonair_user_zdotdir/.zshrc" ] && source "$_labonair_user_zdotdir/.zshrc"
  unset _labonair_user_zdotdir
}

if [[ -z "$__LABONAIR_HOOKS_LOADED" ]]; then
  __LABONAIR_HOOKS_LOADED=1
  autoload -Uz add-zsh-hook 2>/dev/null

  _labonair_urlencode() {
    emulate -L zsh
    setopt localoptions no_multibyte
    local LC_ALL=C s="$1" i byte
    for (( i=1; i<=${#s}; i++ )); do
      byte="${s[i]}"
      case "$byte" in
        [a-zA-Z0-9/._~-]) printf '%s' "$byte" ;;
        *) printf '%%%02X' "'$byte" ;;
      esac
    done
  }

  _labonair_precmd() {
    local _labonair_ret=$?
    printf '\e]133;D;%s\e\\' "$_labonair_ret"
    printf '\e]7;file://%s%s\e\\' "${HOST}" "$(_labonair_urlencode "$PWD")"
    printf '\e]0;\a'
    if [[ -n "$LABONAIR_BLOCKS" ]]; then
      if [[ -n "$_labonair_block_seen" ]]; then
        PS1=$'\n\n%{\e]133;B\e\\%}'
      else
        PS1=$'\n%{\e]133;B\e\\%}'
      fi
      RPROMPT=''
    elif [[ "$PS1" != *$'\e]133;B\e\\'* ]]; then
      PS1=$'%{\e]133;B\e\\%}'"$PS1"
    fi
    printf '\e]133;A\e\\'
  }

  _labonair_preexec() {
    if [[ -n "$LABONAIR_BLOCKS" ]]; then
      _labonair_block_seen=1
      printf '\e]133;C;%s\e\\' "${${1//[[:cntrl:]]/ }[1,256]}"
    else
      printf '\e]133;C\e\\'
    fi
  }

  if (( $+functions[add-zsh-hook] )); then
    add-zsh-hook precmd _labonair_precmd
    add-zsh-hook preexec _labonair_preexec
  fi
  _labonair_precmd
fi
:
