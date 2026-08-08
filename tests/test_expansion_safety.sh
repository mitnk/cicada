#!/bin/bash
# Regression tests for word expansion.
#
# Two things are checked that the script tests in `scripts/` cannot check:
#
#   1. no case may hang -- every run is under a watchdog that kills the whole
#      process group, so a regression fails the suite instead of wedging CI;
#   2. no case may create a file -- expansion output that happens to contain
#      `>` is data, and a test that only compared stdout would miss the
#      clobbering.
#
# Cases are given to `cicada -c` as a single argv string, without an outer
# shell, so the text cicada sees is exactly the text written here.

CICADA="./target/debug/cicada"

if [ ! -f "$CICADA" ]; then
    echo "cicada binary not found: $CICADA"
    echo "please build it out with cargo build first"
    exit 1
fi

CICADA="$(cd "$(dirname "$CICADA")" && pwd -P)/$(basename "$CICADA")"
DIR_WORK="$(mktemp -d "${TMPDIR:-/tmp}/cicada-expansion-test.XXXXXX")"
FILE_OUT="${DIR_WORK}/.stdout"
TIMEOUT_SECS=5

# Files the cases read from. Their contents are the payloads: text that would
# be taken for syntax, or for a regex replacement template, if expansion output
# were rescanned.
printf 'a>PWNED\n'              > "${DIR_WORK}/payload.txt"
printf 'PAYLOAD$head-INJECTED\n' > "${DIR_WORK}/template.txt"
printf '[$0]\n'                 > "${DIR_WORK}/dollar-zero.txt"
printf 'INPUTDATA\n'            > "${DIR_WORK}/input.txt"
SEEDED="dollar-zero.txt input.txt payload.txt template.txt"

passed=0
failed=0

# Run `cicada -c "$1"` with its own process group, so that a case which spins
# can be killed along with anything it started. Returns 124 on timeout.
run_guarded() {
    set -m
    (cd "$DIR_WORK" && "$CICADA" -c "$1") > "$FILE_OUT" 2>&1 &
    local pgid=$!
    set +m

    local ticks=0
    local limit=$((TIMEOUT_SECS * 10))
    while kill -0 "$pgid" 2>/dev/null; do
        if [ "$ticks" -ge "$limit" ]; then
            kill -9 -"$pgid" 2>/dev/null || kill -9 "$pgid" 2>/dev/null
            wait "$pgid" 2>/dev/null
            return 124
        fi
        sleep 0.1
        ticks=$((ticks + 1))
    done

    wait "$pgid"
}

# Any file that the case created, as a sorted one-line list.
files_created() {
    local name
    for name in $(cd "$DIR_WORK" && ls -A | sort); do
        case " $SEEDED .stdout " in
            *" $name "*) ;;
            *) printf '%s ' "$name" ;;
        esac
    done
}

report_failure() {
    failed=$((failed + 1))
    echo "FAIL: $1"
    printf '  script:   %s\n' "$2"
    shift 2
    while [ $# -gt 0 ]; do
        printf '  %s\n' "$1"
        shift
    done
}

# check <label> <script> <expected stdout>
#
# Passes when the case terminates, prints exactly the expected text, and
# creates no file.
check() {
    local label="$1" script="$2" want="$3"

    rm -f "${DIR_WORK}/.stdout"
    local name
    for name in $(files_created); do
        rm -rf "${DIR_WORK:?}/${name}"
    done

    run_guarded "$script"
    local rc=$?
    local got
    got="$(cat "$FILE_OUT")"
    local created
    created="$(files_created)"

    if [ "$rc" -eq 124 ]; then
        report_failure "$label" "$script" "did not terminate within ${TIMEOUT_SECS}s"
        return
    fi
    if [ "$got" != "$want" ]; then
        report_failure "$label" "$script" "expected: $(printf '%q' "$want")" \
            "got:      $(printf '%q' "$got")"
        return
    fi
    if [ -n "$created" ]; then
        report_failure "$label" "$script" "created file(s): ${created}"
        return
    fi

    passed=$((passed + 1))
}

echo '--- expansion terminates ---'
# Parameter expansion forms cicada does not implement print their source text.
# They may not spin, whichever of the two they do.
check '${V:-default} terminates'  'V=set; echo ${V:-default}' '${V:-default}'
check '${V-default} terminates'   'V=set; echo ${V-default}'  '${V-default}'
check '${V:=default} terminates'  'V=set; echo ${V:=default}' '${V:=default}'
check '${V:+alt} terminates'      'V=set; echo ${V:+alt}'     '${V:+alt}'
check '${V%suffix} terminates'    'V=a.txt; echo ${V%.txt}'   '${V%.txt}'
check '${V#prefix} terminates'    'V=a/b; echo ${V#a/}'       '${V#a/}'
check '${#V} keeps its text'      'V=abcd; echo ${#V}'        '${#V}'
check 'a # inside a word'         'echo a#b'                  'a#b'
check 'a # starting a comment'    'echo hi # trailing'        'hi'
check '${V:1:2} terminates'       'V=abcdef; echo ${V:1:2}'   '${V:1:2}'

echo '--- a newline in a value is not rescanned ---'
# The value of `A` ends in a newline and `$HOME` is still unexpanded. Scanning
# the result of an expansion again used to spin here, or drop the prefix.
check 'newline value keeps prefix'  'A="pre
"; echo "${A}post"' 'pre
post'
check 'newline then another $VAR'   'A="pre
"; B=post; echo "$A$B"' 'pre
post'

echo '--- expansion output is data, not more expansion ---'
check 'value with $ stays literal'  "A='\$HOME'; echo \$A"   '$HOME'
check 'braced value with $ literal' "A='\$HOME'; echo \${A}" '$HOME'
check 'unset name expands to empty'  'echo [$no_such_var_xyz]' '[]'

echo '--- command substitution output is literal ---'
# `$head`, `$tail` and `$0` were interpreted as regex replacement references.
# A bare `$0` put the whole `$(...)` back, and it ran again forever.
check '$head in output is literal'  'echo "SAFE-$(cat template.txt)"' 'SAFE-PAYLOAD$head-INJECTED'
check '$0 in output is literal'     'echo "Q$(cat dollar-zero.txt)"'  'Q[$0]'
check '$head via backticks'         'echo "SAFE-`cat template.txt`"'  'SAFE-PAYLOAD$head-INJECTED'

echo '--- command substitution structure ---'
check 'sibling substitutions'   'echo $(echo A)-$(echo B)'       'A-B'
check 'nested substitutions'    'echo $(echo $(echo N))'         'N'
check 'body is a command list'  'echo $(printf a; printf b)'      'ab'
check 'body honours &&'         'echo $(printf a && printf b)'    'ab'
check 'body honours a pipe'     'echo $(echo x | wc -l | tr -d " ")' '1'

echo '--- generated operators stay data ---'
# Each of these would redirect, pipe, or background if the bytes expansion
# produced were compared with operator spellings.
check 'value containing >'      "V='a>OUT'; echo \$V"             'a>OUT'
check 'output containing >'     'echo $(cat payload.txt)'          'a>PWNED'
check 'backtick output with >'  'echo x`cat payload.txt`'          'xa>PWNED'
check 'value is exactly |'      "P='|'; echo a \$P b"              'a | b'
check 'value is exactly <'      "P='<'; echo a \$P input.txt"      'a < input.txt'
check 'value is exactly <<<'    "P='<<<'; echo a \$P input.txt"    'a <<< input.txt'
check 'value is exactly &'      "P='&'; echo a \$P"                'a &'

echo '--- a paren inside quotes is not a delimiter ---'
check 'paren in double quotes'  'echo $(echo "a)b")'          'a)b'
check 'paren in single quotes'  "echo \$(echo 'a)b')"         'a)b'
check 'paren in a nested body'  'echo $(echo $(echo "x)y"))'  'x)y'

echo '--- a marked word is still an assignment ---'
# The marker that keeps generated `<>|&` out of syntax must not also hide an
# assignment: `A=$B cmd` sets `A` for `cmd` whatever `B` holds.
check 'A=$B cmd, value has >'   "B='x>y'; A=\$B printenv A"   'x>y'
check 'A=$B cmd, plain value'   "B='xy'; A=\$B printenv A"    'xy'
check 'A=$B cmd, value has |'   "B='x|y'; A=\$B printenv A"   'x|y'
# A word the user quoted is a command name, not an assignment.
check 'quoted "A=1" is a command' '"A=1" printenv A 2>/dev/null; echo "rc=$?"' 'rc=127'

echo '--- quoted operators stay data ---'
check "quoted '<'"     "echo a '<' b"     'a < b'
check "quoted '<<<'"   "echo a '<<<' b"   'a <<< b'
check "quoted '|'"     "echo a '|' b"     'a | b'
check "quoted final &" "echo done '&'"    'done &'

echo '--- a newline separates commands ---'
check '-c runs both lines'   'echo one
echo two' 'one
two'
check '-c skips blank lines' 'echo one

echo two' 'one
two'
check 'newline inside $()'   'echo $(printf a
printf b)' 'ab'
check 'newline inside quotes' 'echo "one
two"' 'one
two'

echo '--- source operators still work ---'
# The protection above must not disarm redirection that is written in the
# source, so these cases do create files and are checked separately.
run_source_case() {
    local label="$1" script="$2" want="$3"
    run_guarded "$script"
    local rc=$? got
    got="$(cat "$FILE_OUT")"
    if [ "$rc" -eq 124 ]; then
        report_failure "$label" "$script" "did not terminate within ${TIMEOUT_SECS}s"
    elif [ "$got" != "$want" ]; then
        report_failure "$label" "$script" "expected: $(printf '%q' "$want")" \
            "got:      $(printf '%q' "$got")"
    else
        passed=$((passed + 1))
    fi
}

run_source_case 'redirect to a literal name' 'echo hi > out1.txt; cat out1.txt' 'hi'
run_source_case 'redirect to an expansion'   'F=out2.txt; echo hi > $F; cat out2.txt' 'hi'
run_source_case 'append to a file'           'echo a > out3.txt; echo b >> out3.txt; cat out3.txt' 'a
b'
run_source_case 'read from a file'           'cat < input.txt' 'INPUTDATA'
run_source_case 'here-string'                'cat <<< hello' 'hello'
run_source_case 'pipe'                       'echo hi | wc -l | tr -d " "' '1'
run_source_case 'stderr to stdout'           'ls no-such-file-xyz 2>&1 | wc -l | tr -d " "' '1'

rm -rf "$DIR_WORK"

echo
echo "expansion safety: ${passed} passed, ${failed} failed"
[ "$failed" -eq 0 ] || exit 1
echo OK.
