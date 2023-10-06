#!/usr/bin/env sh

# Makes sure no db files exist before running all the test.
if [ -f "db.qkv" ]; then
  rm db.qkv
fi

cargo test --features full

if [ -f "db.qkv" ]; then
  rm db.qkv
fi