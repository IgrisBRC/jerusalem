#!/bin/bash

# Find all .rs files in src, excluding hidden files
find src -name "*.rs" | while read -r file; do
    echo "### FILE: $file"
    echo '```rust'
    cat "$file"
    echo '```'
    echo ""
done
