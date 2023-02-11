export FOO=foo
echo "A${FOO}B$FOO/C${FOO}D"
echo "A$FOO/B${FOO}C$FOO/D"

# test cd/pwd on the absolute path string
mkdir -p foo1/bar1
cd foo1/bar1
cd ..
pwd | awk -F/ '{print $NF}'
cd ..
rm -rf ./foo1
echo =1=
yes | cat | cat | head -n 999999 | head -n 1
echo =2=

# test env var starts with _
_foo=135
echo $_foo
unset _foo
echo "=${_foo}="

# test
(echo "ignore parens 1")
(echo 'ignore parens 2' | cat)
(echo foop1)
(echo "foop2")
(echo "foop3")
export fOOp4="(foop4)"
echo $fOOp4
echo "---"

# commands from vim e.g. !sort
echo c5 > .sort-input
echo a1 >> .sort-input
echo b3 >> .sort-input
(sort) < .sort-input > .sort-output
cat .sort-output
rm -f .sort-input .sort-output
