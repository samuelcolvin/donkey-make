# donk bash-completion script

# to install bash completion for donk, add the following to ~/.bashrc or equivalent:
#     eval "$(_DONK_COMPLETE=source donk --completion-script)"

_donk()
{
  local cur prev words cword split
  _init_completion -s || return

  case $prev in
    -h|--help|-V|--version|--completion-script)
      return
      ;;
    -f|--file)
      _filedir y*ml
      return
      ;;
  esac

  $split && return

  if [[ $cur == -* ]]; then
    # --completion-script is not added here, since completion must already be installed to be running this
    COMPREPLY=($(compgen -W "-f --file -k --keep-tmp-file --help --version" -- "$cur" ))
  elif hash donk; then
    COMPREPLY=($(compgen -W "$(donk --complete-command $prev)" -- "$cur"))
  fi
} &&
complete -F _donk donk

# to install bash completion for donk, add the following to ~/.bashrc or equivalent:
#     eval "$(_DONK_COMPLETE=source donk --completion-script)"
