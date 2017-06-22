#!/bin/bash

set -e

echo "Removing old docs"
rm -rf target/doc

echo "Compile docs"
cargo doc -p scheduled-executor --no-deps
echo '<meta http-equiv=refresh content=0;url=scheduled_executor/index.html>' > target/doc/index.html

echo "Run ghp-import"
ghp-import -n target/doc

echo "Push to github"
git push origin gh-pages

# Squash changes: git rebase --root -i
