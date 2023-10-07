#!/usr/bin/env sh

cd ./target/debug

# If no arguments are passed, error
# if [ $# -eq 0 ]; then
#     echo "Please pass arguments to the program."
#     exit 1
# fi

./quick-kv-cli "$@"
