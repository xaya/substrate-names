#!/bin/bash -e

function stop_container() {
  echo "Trapped SIGINT, stopping..."
  exit 0
}
trap stop_container INT

case $1 in
  node)
    shift
    exec node --base-path="/var/lib/node" "$@"
    ;;

  frontend)
    nginx -g "daemon off;"
    ;;

  *)
    echo "Provide either \"node\" or \"frontend\" as command"
    exit 1
esac
