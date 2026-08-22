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
printf 'one $(touch DOLLAR-RAN) two\n' > "${DIR_WORK}/cmdsub.txt"
printf 'one `touch TICK-RAN` two\n'    > "${DIR_WORK}/backtick.txt"
SEEDED="backtick.txt cmdsub.txt dollar-zero.txt input.txt payload.txt template.txt"

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

echo '--- an escaped dollar stays a dollar ---'
# `\$` is text, whatever follows it. The `$` is marked as data in the parser
# rather than left behind a backslash, so no later step has to guess: a name
# that expansion knows (`\$b`), one it does not (`\$1`), and one that is not a
# name at all (`\$$`) all come out the same way.
check 'escaped dollar in a word'         'echo a\$b'      'a$b'
check 'escaped dollar in double quotes'  'echo "a\$b"'    'a$b'
check 'escaped dollar at end'            'echo a\$'       'a$'
check 'escaped dollar at word start'     'echo \$b'       '$b'
check 'escaped dollar in single quotes'  $'echo \'a\\$b\'' 'a\$b'
check 'escaped dollar before a digit'    'echo a\$1'      'a$1'
check 'escaped dollar before a zero'     'echo a\$0'      'a$0'
check 'escaped braced digit'             'echo a\${1}'    'a${1}'
check 'escaped braced name'              'echo \${HOME}'  '${HOME}'
check 'escaped dollar then dollar'       'echo a\$\$'     'a$$'
check 'escaped dollar then question'     'echo a\$\?'     'a$?'
check 'escaped and live dollar in a word' 'A=B; echo a\$1$A' 'a$1B'
check 'escaped dollar before a path'     'echo \$HOME/x'  '$HOME/x'
check 'escaped dollar sub is not run'    'echo "\$(touch DOLLAR-RAN)"' '$(touch DOLLAR-RAN)'
check 'escaped dollar through a sub'     'echo $(echo a\$b)' 'a$b'
check 'escaped ampersand still works'    'echo x\&y'      'x&y'
check 'escaped double quote still works' 'echo a\"b'      'a"b'

echo '--- expansion output is not run as a command ---'
# A value is data. Substitution syntax that only appears once a value has been
# put in place must not run: these cases carry the payload in from a file, the
# way a script reading untrusted input would.
check 'value with $(...) not run'   'V=$(cat cmdsub.txt); echo "[$V]"'   '[one $(touch DOLLAR-RAN) two]'
check 'value with backticks not run' 'V=$(cat backtick.txt); echo "[$V]"' '[one `touch TICK-RAN` two]'
check 'value $(...) as an argument'  'V=$(cat cmdsub.txt); printf "%s\n" "$V"' 'one $(touch DOLLAR-RAN) two'
check 'value with $(...) unquoted'   'V=$(cat cmdsub.txt); echo $V'      'one $(touch DOLLAR-RAN) two'

echo '--- double quotes do not hide a substitution ---'
# Only single quotes make a substitution literal. These ran as expected once,
# then stopped when a regex was added to keep `V='$(cmd)'` from running.
check 'quoted sub in an assignment'  'V="$(echo hi)"; echo $V'          'hi'
check 'quoted sub with text around'  'V="x$(echo hi)y"; echo $V'        'xhiy'
check 'backticks in double quotes'   'V="`echo hi`"; echo $V'           'hi'
check 'apostrophes around a sub'     "echo \"a='\$(echo hi)'\""       "a='hi'"
check 'apostrophe in a quoted word'  "echo \"it's \$(echo now)\""      "it's now"

echo '--- single quotes hold in an assignment ---'
# The quoting is inside the word here, which is where a scanner that only
# looks at the word's outer quote marker stops seeing it.
check 'backticks in a quoted value'  "V='\`touch TICK-RAN\`'; echo \"[\$V]\"" '[`touch TICK-RAN`]'
check 'dollar sub in quoted value'   "V='\$(touch DOLLAR-RAN)'; echo \"[\$V]\"" '[$(touch DOLLAR-RAN)]'
check 'backticks quoted, unused'     "V='\`touch TICK-RAN\`'; echo done" 'done'

echo '--- quotes in an assignment value are removed ---'
# Quote removal is the last step of expansion, not the parser's job: the
# quotes have to still be in the word while substitution decides what is
# quoted, and they have to be gone by the time the value is stored.
check 'double quotes inside assignment value'  'V=a"x"c; echo $V'  'axc'
check 'single quotes inside assignment value'  "V=a'x'c; echo \$V"  'axc'
check 'double quoted assignment value'         'V="x"; echo $V'     'x'
check 'single quoted assignment value'         "V='x'; echo \$V"     'x'
check 'double quotes in ordinary word'         'echo a"x"c'         'axc'
check 'several quoted stretches'      'V=a"x"c"y"d; echo $V'        'axcyd'
check 'a space inside the quotes'     'V=a"b c"; echo "[$V]"'       '[ab c]'
check 'quotes around an expansion'    'A=1; V=a"$A"b; echo "[$V]"'  '[a1b]'
check 'quote removal on export'       'export V=a"x"c; echo "[$V]"' '[axc]'
check 'empty quoted stretch'          'V=a""b; echo "[$V]"'         '[ab]'

echo '--- a quote that is not syntax is kept ---'
# Only the quotes the *line* was written with are removed. One that a value
# carried in, or one that was escaped, is a character of the value: removing
# it would corrupt data on its way through an assignment.
check 'escaped double quote in a value' 'V=a\"b; echo "[$V]"'  '[a"b]'
check 'escaped single quote in a value' $'V=a\\\'b; echo "[$V]"' "[a'b]"
check 'unpaired escaped quote'          'V=x\"; echo "[$V]"'   '[x"]'
check 'apostrophe inside double quotes' $'V="it\'s"; echo "[$V]"' "[it's]"
check 'a double quote in a value'       "B=\$(echo 'x\"y'); A=\$B; echo \"[\$A]\"" '[x"y]'
check 'an apostrophe in a value'        "B=\$(echo \"x'y\"); A=\$B; echo \"[\$A]\"" "[x'y]"
check 'a quote in a value, exported'    "B=\$(echo 'x\"y'); A=\$B env | grep '^A='" 'A=x"y'
check 'a backslash in a value'          "B='a\\b'; A=\$B; echo \"[\$A]\""  '[a\b]'
check 'quotes in substitution output'   "echo \$(echo '\"x\"')"   '"x"'

echo '--- an escaped operator stays text ---'
# An operator character is only an operator when the line wrote it as one.
# These three each hid one from a different place, and each broke a different
# way when the hiding place stopped being checked.
check 'escaped redirect in a value'  'V="a\>b"; echo "[$V]"'    '[a\>b]'
check 'escaped redirect, unquoted'   'echo a\>b'               'a>b'
check 'quoted redirect in a value'   "export E1=' >'; echo \"[\$E1]\"" '[ >]'
check 'quoted pipe in a value'       'V="a|b"; echo "[$V]"'     '[a|b]'
check 'quoted amp in a value'        "V='a&b'; echo \"[\$V]\""  '[a&b]'
check 'adjacent quoted stretches'    $'echo \'a\'\'b\''      'ab'
check 'escapes in a second stretch'  $'echo \'a\'\'\\\\b\''    'a\\b'

echo '--- a prompt keeps its own syntax, not its quotes ---'
# `export PROMPT=...` skips expansion: the `$` sequences belong to the prompt
# and are read when it is drawn. Quote removal still has to run, or the prompt
# is drawn with the quotes it was written with.
check 'prompt keeps its names'   'export PROMPT="${GREEN}x${RESET} "; echo "[$PROMPT]"' '[${GREEN}x${RESET} ]'
check 'prompt loses its quotes'  'export PROMPT="a b"; echo "[$PROMPT]"'   '[a b]'
check 'prompt quoted mid-value'  'export PROMPT=a"x"c; echo "[$PROMPT]"'   '[axc]'
check 'prompt in single quotes'  "export PROMPT='\${G}x '; echo \"[\$PROMPT]\"" '[${G}x ]'
check 'prompt with escaped $'    'export PROMPT="\$ "; echo "[$PROMPT]"'   '[$ ]'

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
