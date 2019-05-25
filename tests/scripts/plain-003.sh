# regression tests
echo "a\"b"

echo 'cicada, is not a "cicada", but a "unix shell".' \
    | awk -F "[ ,.\"]+" '{for(i=1;i<=NF;i++)A[$i]++}END{for(k in A)print k, A[k]}' \
    | sort -k2nr \
    | head -n5
