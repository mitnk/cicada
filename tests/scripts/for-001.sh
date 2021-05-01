echo hi
for var in foo bar baz
    echo $var
done

for var2 in $(echo a b)
    echo hello && echo $var2
done

for var3 in 'args kwargs' "sh script"
    echo $var3
done

for var4 in src/builtins/ex*.rs
    echo source file $var4
done

for c in ab , ' ' xy
    echo "append '$c' into file"
done

echo bye

echo ===1===

echo hi
for xxx in foo bar baz
    echo $xxx
done

for xxx in $(echo a b)
    echo hello && echo $xxx
done

for xxx in 'args kwargs' "sh script"
    echo $xxx
done

echo bye

echo ===2===

echo before for-if
for xxx in foo
    if echo $xxx | grep -iq foo
        echo found it
    fi
done
echo after for-if
echo

echo before for-while
counter=18
for xxx in foo bar
    echo hello $xxx
    while echo "$counter" | grep -iq "^1.$"
        echo "counter = $counter"
        counter=$(expr $counter + 1)
    done
    echo bye $xxx
done
echo after for-while

echo ===3===

# rest $xxx
xxx=

while echo $xxx | grep -iqv "final"
    echo "xxx begins with $xxx"
    for xxx in wip final
        echo xxx now is $xxx
    done
done

echo ===4===

for x in foo bar
        continue
done

for x in 1 2 3
    if [ "$x" = "2" ]
        continue
    fi
    echo $x
done

for x in 4 5 6
    if [ "$x" = "4" ]
        if [ "$x" = "4" ]
            continue
        fi
    fi
    echo $x
done

for x in 8 9 10
    if [ "$x" = "10" ]
        if [ "$x" = "10" ]
            continue
        fi
    fi
    echo $x
done

for x in a b
    if [ "$x" = "c" ]
        continue
    fi
    echo $x
done

echo ===5===

echo zero round
for x in 1 2 3
    for y in a b c
        echo "$y$x"
    done
done

echo first round
for x in 1 2 3
    for y in a b c
        if [ "$x" = "2" ]
            continue
        fi
        echo "$y$x"
    done
done

echo second round
for x in 1 2 3
    for y in a b c
        if [ "$y" = "b" ]
            continue
        fi
        echo "$y$x"
    done
done

echo ===6===

for x in foo bar baz
    echo $x
    break
done

echo first round
for x in 1 2 3
    for y in a b c
        if [ "$x" = "2" ]
            break
        fi
        echo "$y$x"
    done
done

echo second round
for x in 1 2 3
    for y in a b c
        if [ "$y" = "b" ]
            break
        fi
        echo "$y$x"
    done
done

echo ===7===

for x210501 in foo bar; do
    echo "-" $x210501
done

echo ===8===
