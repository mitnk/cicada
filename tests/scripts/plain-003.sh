# regression tests
echo "a\"b"

echo 'cicada, is not a "cicada", but a "unix shell".' \
    | awk -F "[ ,.\"]+" '{for(i=1;i<=NF;i++)A[$i]++}END{for(k in A)print k, A[k]}' \
    | sort -k2nr \
    | head -n5

echo {a,b}-$nosuchenv
echo {a,b}-${nosuchenv}

# tests on ~ expansion
touch ~/283b812a.txt
ls ~ 2>/dev/null | grep -o 283b812a
ls ~/ 2>/dev/null | grep -o 283b812a
rm -f ~/283b812a.txt
