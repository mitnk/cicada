plain_counter=35
echo hi
echo mid > /dev/null

echo hello \
       world

echo foo\
bar

echo hello world \
    | sed 's/hello/hi/' \
    | wc \
    | sed -e 's/  */ /g' \
          -e 's/^  *//'

echo bye "counter $plain_counter"

echo ==1==

if echo hi > /dev/null
    findext 2>/dev/null || echo findext not found
fi

if echo hi > /dev/null
    ifif7 2>/dev/null || echo ifif7 not found
fi

for x in foo
    ford 2>/dev/null || echo ford not found
done

for x in foo
    doned 2>/dev/null || echo doned not found
done

counter=19
while echo "$counter" | grep -iq "^1.$"
    whiled 2>/dev/null || echo whiled not found
    counter=$(expr $counter + 1)
done

echo ==2==

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

cd
echo 283*

rm -f ~/283b812a.txt

touch 2019-07-29
touch 2019-07-30
echo 2019*
echo 2019-*
echo 2019-0*
echo 2019-07*
echo 2019-07-*
echo 2019-07-2*
echo 2019-07-3*
rm -f 2019-07-29
rm -f 2019-07-30

31415926 + 1 
3 - 1
4 * 5
4 / 5.0 
2 ^ 5
((1 + 1) ^ (5 + 1) )

# cicada: 31415926: command not found
31415926 2>&1 | cat

echo ==3==

touch 'foo.1' 'bar.1' 'foo.txt'
ls *.1 | sort

ls f* | sort

echo 'f*'
echo "bar*"

rm -f 'foo.1' 'bar.1' 'foo.txt'

echo ==4==

# test ENV inside command sup
echo $(echo foo bar | awk '{print $NF}')
echo $(echo foo bar z | awk '{print $NF}')
echo `echo foo bar1 | awk '{print $NF}'`
echo `echo foo bar z2 | awk '{print $NF}'`

VER1=`echo foo bar baz | awk '{print $NF}'`; echo $VER1
VER2=$(echo foo bar baz3 | awk '{print $NF}'); echo $VER2
VER3=$(echo foo bar baz4 | awk "{print $NF}"); echo $VER3
VER4=`echo foo bar baz5 | awk '{print $NF}'`; echo $VER4
VER5=`echo foo bar baz6 | awk "{print $NF}"`; echo $VER5

echo ==5==

echo test right head commands finishes first
yes | head -n 2
yes | head -n 2 | cat
yes | head -n 2 | cat | head | head

echo ==6==
