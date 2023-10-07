#!/bin/bash

# Check if the current directory is a Rust crate
if ! cargo metadata --no-deps --format-version 1 >/dev/null 2>&1; then
  echo "Error: Not a Rust crate directory. Please run this script in your crate's root directory."
  exit 1
fi

# If not formatted, some cargo commands will fail, this is just a safely check
./scripts/fmt-project.sh

# Makes sure no db files exist before running all the test.
if [ -f "db.qkv" ]; then
  rm db.qkv
fi

# Check if cargo fmt reports any formatting issues
if ! cargo +nightly fmt --all -- --check; then
  echo "Error: Formatting issues detected. Please run 'cargo fmt' to format your code."
  exit 1
fi

# Check if clippy lints report any issues
if ! cargo clippy --all -- -D warnings; then
  echo "Error: Clippy lints detected issues. Please fix the reported warnings and errors."
  exit 1
fi

# Generate documentation to check for issues
if ! cargo doc --no-deps --all --release; then
  echo "Error: Failed to generate documentation. Please fix any doc comments or build issues."
  exit 1
fi

# Check if all doc tests pass
# if ! cargo test --doc --all; then
#   echo "Error: Some documentation tests failed. Please ensure doc comments are correct."
#   exit 1
# fi

# Check if cargo publish would succeed (dry run)
if ! cargo publish --dry-run --allow-dirty; then
  echo "Error: 'cargo publish' dry run failed. Please fix any issues reported."
  exit 1
fi

if [ -f "db.qkv" ]; then
  rm db.qkv
fi

echo "Everything looks good! You can proceed with 'cargo publish' to publish your crate."
exit 0