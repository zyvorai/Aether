#!/bin/sh
# Test double for the podman and docker CLIs, with the rules that matter to
# Aether: names are unique, a stopped container keeps its name, IDs survive a
# rename. State lives under $FAKE_CONTAINERS:
#   c/<name>    status|health|restarts
#   meta/<name> id|managed
# While the file `fail_run` exists, `run` creates the container and then fails,
# as an engine does when it cannot start it (for example, a port is taken).
D=${FAKE_CONTAINERS:?}
mkdir -p "$D/c" "$D/meta"

# Print the name of the container known by this name or ID.
find_name() {
  if [ -f "$D/c/$1" ]; then
    echo "$1"
    return
  fi
  for f in "$D"/meta/*; do
    [ -f "$f" ] || continue
    if [ "$(cut -d'|' -f1 "$f")" = "$1" ]; then
      basename "$f"
      return
    fi
  done
  return 1
}

none() {
  echo "Error: no such container $1" >&2
  exit 125
}

cmd=$1
shift
[ "$cmd" = container ] && { cmd=$1; shift; }
case $cmd in
  build) exit 0 ;;
  run)
    name=
    managed=
    while [ $# -gt 0 ]; do
      [ "$1" = --name ] && name=$2
      [ "$1" = aether-managed=true ] && managed=true
      shift
    done
    if [ -f "$D/c/$name" ]; then
      echo "Error: name \"$name\" is already in use" >&2
      exit 125
    fi
    n=$(($(cat "$D/serial" 2>/dev/null || echo 0) + 1))
    echo "$n" > "$D/serial"
    echo "id$n|$managed" > "$D/meta/$name"
    if [ -f "$D/fail_run" ]; then
      echo "created|none|0" > "$D/c/$name"
      echo "Error: fake run failure" >&2
      exit 126
    fi
    echo "running|none|0" > "$D/c/$name"
    echo "id$n"
    ;;
  stop | start)
    name=$(find_name "$1") || none "$1"
    if [ "$cmd" = stop ]; then state=exited; else state=running; fi
    echo "$state|none|0" > "$D/c/$name"
    ;;
  rename)
    name=$(find_name "$1") || none "$1"
    if [ -f "$D/c/$2" ]; then
      echo "Error: name \"$2\" is already in use" >&2
      exit 125
    fi
    mv "$D/c/$name" "$D/c/$2"
    mv "$D/meta/$name" "$D/meta/$2"
    ;;
  rm)
    for a; do
      case $a in
        -*) ;;
        *)
          name=$(find_name "$a") || none "$a"
          rm -f "$D/c/$name" "$D/meta/$name"
          ;;
      esac
    done
    ;;
  inspect)
    format=
    ref=
    while [ $# -gt 0 ]; do
      case $1 in
        --format)
          format=$2
          shift
          ;;
        *) ref=$1 ;;
      esac
      shift
    done
    name=$(find_name "$ref") || none "$ref"
    case $format in
      *Labels*) echo "$(cut -d'|' -f1 "$D/c/$name")|$(cut -d'|' -f2 "$D/meta/$name")" ;;
      *) cat "$D/c/$name" ;;
    esac
    ;;
  logs) echo "fake log line" ;;
  ps) echo "[]" ;;
  *)
    echo "fake container cli: unsupported $cmd" >&2
    exit 125
    ;;
esac
