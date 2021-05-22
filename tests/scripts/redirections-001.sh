# no stdout
echo hi1 > /dev/null

# stdout: 0
echo hi2 1>&2 | wc -l | awk '{print $1}'

# no stdout all these files do not exists
ls -l file-1.txt 2>/dev/null | cat

# stdout: file-2.txt
ls -l file-2.txt 2>&1 | grep -o '\<file...txt\>'

# stdout: 0
ls -l file-3.txt 2>&1 | cat 1>&2 | wc | awk '{print $1}'
echo ==1==

cat <<< hello
cat <<< 'foo bar'
cat <<< "$(3 + 4)"
cat <<< a | wc
cat <<< a | wc | wc
cat <<< hello | wc <<< a | wc
cat <<< a | wc > /dev/null | wc

echo ==2==

# test builtin redirections
echo check minfd 1
# this would be 4, since 3 is occupied by file of current script
minfd  # check min fd

alias foo='echo 135'
alias foo

echo no output
alias foo >/dev/null

alias foo >/dev/null 2>&1 > foo-alias.txt
echo result in file
cat foo-alias.txt
rm -f foo-alias.txt

echo check minfd 2
minfd  # check min fd

echo ==3==

alias bar-not-exist  # not found, print to stderr
alias bar-not-exist >/dev/null 2>&1  # not output at all

alias bar-not-exist >bar-alias.txt 2>&1  # not output at all
rm -f bar-alias.txt

alias bar-not-exist 2>bar1-alias.txt 2>bar2-alias.txt
echo check bar1
cat bar1-alias.txt
echo check bar2
cat bar2-alias.txt
echo after check bar2

rm -f bar*-alias.txt

# run some random pipeline before minfd to cover pipes closing
echo hi | wc -l | wc | cat | cat >/dev/null 2>&1

echo check minfd err
minfd  # check min fd

echo ==4==

alias sec5_1="echo xsec51"
echo one alias
alias sec5_1

echo one alias with grep
alias sec5_1 | grep -o xsec51

echo all alias with grep
alias | grep -o xsec51

echo builtin alias in mid
echo hi | alias | grep -o xsec51

echo check minfd err
minfd  # check min fd

echo ==5==
