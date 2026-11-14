# The player's home (sourced by run.sh and session.sh): the working directory of every play
# session. It sits outside the repository with no git above it and holds only journal/, so
# the session sees no repository files, commits or project notes.
#   ULTRON_PLAY_HOME overrides the default ~/.local/state/ultron-play.

PLAY_HOME="${ULTRON_PLAY_HOME:-$HOME/.local/state/ultron-play}"

# Checks that the home is usable: not inside the repository and not inside any git tree.
check_home() {
  local home="$1" repo="$2" real
  mkdir -p "$home" || return 1
  real="$(realpath "$home")"
  case "$real/" in
    "$(realpath "$repo")"/*) echo "play home $real is inside the repository" >&2; return 1 ;;
  esac
  if git -C "$real" rev-parse --git-dir > /dev/null 2>&1; then
    echo "play home $real is inside a git work tree" >&2; return 1
  fi
}

# Moves the journal from the repository into the home once and leaves a link in its place,
# so journal/ in the repository still reaches the same files. Refuses to merge two
# non-empty journals.
adopt_journal() {
  local repo_j="$1" home="$2"
  mkdir -p "$home/journal" || return 1
  [[ -L "$repo_j" ]] && return 0
  if [[ -d "$repo_j" ]]; then
    if [[ -n "$(ls -A "$repo_j")" ]]; then
      if [[ -n "$(ls -A "$home/journal")" ]]; then
        echo "both $repo_j and $home/journal hold files; merge them by hand" >&2
        return 1
      fi
      mv "$repo_j"/* "$repo_j"/.[!.]* "$home/journal/" 2> /dev/null
      [[ -z "$(ls -A "$repo_j")" ]] || { echo "could not move $repo_j" >&2; return 1; }
    fi
    rmdir "$repo_j" || return 1
  fi
  ln -s "$home/journal" "$repo_j"
}
