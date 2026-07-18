#!/usr/bin/env python3
"""Read and output file content with backticks escaped for safe terminal output."""
import sys
fname = sys.argv[1]
with open(fname, 'r', encoding='utf-8') as f:
    content = f.read()
# Escape backticks to prevent terminal parsing issues
content = content.replace('`', '\x60')
print(content)
