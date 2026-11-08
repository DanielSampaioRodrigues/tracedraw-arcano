#!/usr/bin/env bash
# Fails when the lines added since BASE (the first argument) break two of
# the golden rules in AGENTS.md that tests cannot see: the names of other
# graphics programs (rule 3; the list is kept outside the repository, in
# the FORBIDDEN_WORDS variable of the CI settings, so the repository does
# not carry the names itself) and em dashes (rule 8).
set -euo pipefail
base="${1:?usage: check_added_lines.sh BASE}"
added=$(git diff --unified=0 "$base" HEAD -- . ':(exclude)ATTRIBUTION.md' \
    ':(exclude)CLEANROOM.md' ':(exclude)README.md' ':(exclude)PRIVACY.md' \
    | grep -E '^\+[^+]' || true)
status=0
if [ -n "${FORBIDDEN_WORDS:-}" ]; then
    hits=$(printf '%s\n' "$added" | grep -inE "$FORBIDDEN_WORDS" || true)
    if [ -n "$hits" ]; then
        echo "Added lines name another graphics program (AGENTS.md rule 3):"
        printf '%s\n' "$hits"
        status=1
    fi
fi
dashes=$(printf '%s\n' "$added" | grep -n $'\xe2\x80\x94' || true)
if [ -n "$dashes" ]; then
    echo "Added lines use an em dash (AGENTS.md rule 8):"
    printf '%s\n' "$dashes"
    status=1
fi
[ "$status" -eq 0 ] && echo "Added lines: no forbidden names, no em dashes."
exit "$status"
