#!/usr/bin/env sh

# Makes sure no files with .qkv extension exist before running all the tests.
if [ -n "$(find . -maxdepth 1 -name '*.qkv' -print -quit)" ]; then
  find . -maxdepth 1 -name '*.qkv' -type f -delete
fi

# Run your tests here
cargo test --doc --all

# Deletes any remaining .qkv files after the tests.
if [ -n "$(find . -maxdepth 1 -name '*.qkv' -print -quit)" ]; then
  find . -maxdepth 1 -name '*.qkv' -type f -delete
fi
