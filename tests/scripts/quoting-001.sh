# Escaping and quote removal, run as a script.
#
# Script mode has its own way in: a line is parsed, has its `$1`s put in, and
# is then put back together as a line. Cases that behave one way through
# `cicada -c` and another way here are what that round trip breaks, so they
# are worth checking on both roads.

echo "== escapes =="
echo a\$b
echo "a\$b"
echo a\$1
echo a\"b
V=a\"b; echo "[$V]"
echo a\\b
echo "a\\b"

echo "== quote removal in a value =="
V=a"x"c; echo "[$V]"
V=a'x'c; echo "[$V]"
V="a b"c; echo "[$V]"
export E=a"x"c; echo "[$E]"

echo "== single quotes hold =="
V='$HOME'x; echo "[${V}]"
V=x'`echo ran`'; echo "[$V]"
V='$(echo ran)'; echo "[$V]"

echo "== operators stay text =="
V="a\>b"; echo "[$V]"
V="a|b"; echo "[$V]"
export F=' >'; echo "[$F]"

echo "== positional args =="
echo "1=$1"
echo "lit=\$1 real=$1"
echo \$1 $1
V='$HOME'$1; echo "[$V]"
