cat > /tmp/cicada-heredoc-test.txt << XX
1
2
XX
cat >> /tmp/cicada-heredoc-test.txt << end
a
b
end
cat /tmp/cicada-heredoc-test.txt
rm -f /tmp/cicada-heredoc-test.txt

cat <<'EOF' | sed 's/l/e/g'
Hello
World
EOF

cat<<EOF
adjacent unquoted
EOF

FOO=bar
cat << EOF
value is $FOO
EOF

cat <<- TAB
	alpha
	TAB

echo "== expansions in an unquoted body"
NAME=cicada
cat << EOF
name is ${NAME}
subs $(echo one) and `echo two`
escaped \$NAME and \\ backslash
EOF

echo "== a quoted delimiter is literal"
cat << 'EOF'
name is $NAME, subs $(echo one), escaped \$x
EOF

cat << "END"
double quoted $NAME
END

echo "== empty body"
cat << EOF
EOF
echo "after the empty one"

echo "== read builtin"
read line_one << EOF
from a heredoc
EOF
echo "read got: $line_one"

echo "== same heredoc, more than once"
for i in 1 2
    cat << EOF
iter $i
EOF
done

function greet() {
    cat << EOF
hello $1
EOF
}
greet world
greet again

echo "== these are not heredocs"
echo "a << b"
echo 'c << d'
echo hi  # a << EOF inside a comment
cat <<< "here string"

echo "== a backslash-quoted delimiter is literal too"
cat << \EOF
literal $HOME and $(echo sub)
EOF

echo "== a word that only looks like a marker is just a word"
FAKE=__cicada_heredoc_0
cat <<< $FAKE
