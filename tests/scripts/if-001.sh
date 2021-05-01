echo hi

if echo foo | grep -iq o
    echo found foo
fi

if echo foo | grep -iq ar
    echo found bar
fi

if echo foo | grep -iq ar
    echo found foo
else if echo baz | grep -iq az
    echo found baz
else
    echo no foo and no baz
fi

if echo foo | grep -iq a
    echo found a
else if echo foo | grep -iq b
    echo found b
else
    echo no a and no b
fi

echo bye

echo =1=

# else br returns
if echo foo | grep -iq ar
    echo found foo
else
    echo not found  # <- executed
fi
echo bye

echo =2=

echo hi
counter=17
if echo foo | grep -q oo
    while echo "$counter" | grep -iq "^1.$"
        echo "counter = $counter"
        counter=$(expr $counter + 1)
    done
fi

if echo foo | grep -q bar
    # will not enter this if
    while echo "$counter" | grep -iq "^2.$"
        echo "counter = $counter"
        counter=$(expr $counter + 1)
    done
fi

if echo foo | grep -q oo
    if echo bar | grep -q oo
        echo found oo
    else
        while echo "$counter" | grep -iq "^2[0-2]$"
            echo "counter = $counter"
            counter=$(expr $counter + 1)
        done
    fi
fi

if echo foo | grep -q oo
    if echo bar | grep -q bar
        echo found bar
    else
        while echo "$counter" | grep -iq "^2.$"
            echo "counter = $counter"
            counter=$(expr $counter + 1)
        done
    fi
fi

echo bye

echo =3=

# NOTE: this script will print two lines into stderr,
# which will not break the tests
x=foo
if echo hi
    if [ $x = foo ]
        echo before printing continue
        continue
        echo after printing continue
    fi
    echo end of outer if
fi
echo bye

# just a comment
continue

echo bye bye

echo =4=

# NOTE: this script will print two lines into stderr,
# which will not break the tests
x=foo
if echo hi
    if [ $x = foo ]
        echo before printing break
        break
        echo after printing break
    fi
    echo end of outer if
fi
echo bye

# just a comment
break

echo bye bye

echo =5=

if true; then
    echo 'found foo!'
fi

if echo 'true; then' | grep -q then; then
    echo bye
fi

echo =6=
