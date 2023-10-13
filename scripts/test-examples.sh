#!/usr/bin/env sh

if [ -f "db.qkv" ]; then
  rm db.qkv
fi

cargo run --example basic
cargo run --example advanced --features full

if [ -f "db.qkv" ]; then
  rm db.qkv
fi