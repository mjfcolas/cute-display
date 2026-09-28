#!/bin/sh
# The version the next release would take: `<year>.<month>.<n>`, n one past this month's
# highest release tag, 0 in a month without one. Only tags `v<year>.<month>.<n>`, with no
# leading zero, count. `just release` names a snapshot after it. The month may be given,
# as `<year>.<month>`; it is this one otherwise.
set -eu

month=${1:-$(date +%Y.%-m)}
highest=$(git tag --list 'v*' | grep -E '^v[0-9]{4}\.[1-9][0-9]?\.(0|[1-9][0-9]*)$' | sed 's/^v//' \
    | grep -F "$month." | sort -t . -k 3,3n | tail -n 1 || true)
if [ -n "$highest" ]; then
    echo "$month.$(( ${highest##*.} + 1 ))"
else
    echo "$month.0"
fi
