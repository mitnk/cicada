function foo-bar() {
    echo foo
    echo bar
}

function ifif7 {
    echo 27 if printed
}

function what-is-my-args {
    echo $@
    echo $0 $1 $2 $3
}

foo-bar
ifif7

what-is-my-args a b c d
echo current script is $0 | sed 's|\(.*\) [^ ]*\(tests/scripts/func-001.sh\)|\1 \2|'
