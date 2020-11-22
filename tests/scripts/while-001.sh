echo hi
counter=17
while echo "$counter" | grep -iq "^1.$"
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done
echo bye

echo =1=

echo hi
counter=17
while echo "$counter" | grep -iq "^1.$"
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done
echo bye

echo =2=

echo hi

counter=18
while echo "$counter" | grep -iq "^1.$"
    counter=$(expr $counter + 1)

    if echo foo | grep -q foo
        echo found foo $counter
    fi
done

while echo "$counter" | grep -iq "^2[01]$"
    counter=$(expr $counter + 1)

    if echo foo | grep -q bar
        echo found bar
    else
        echo not found $counter
    fi
done

echo bye

echo =3=

counter=17
while echo "$counter" | grep -iq "^1.$"
    if [ $counter = 18 ]
        counter=$(expr $counter + 1)
        continue
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=26
while echo "$counter" | grep -iq "^2.$"
    if [ $counter = 27 ]
        counter=$(expr $counter + 1)
        if [ $counter = 28 ]
            continue
        fi
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=36
while echo "$counter" | grep -iq "^3.$"
    if [ $counter = 37 ]
        counter=$(expr $counter + 1)
        if [ $counter = 38 ]
            counter=$(expr $counter + 1)
            continue
        fi
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

echo =4=

counter=17
while echo "$counter" | grep -iq "^1.$"
    break
done

while echo "$counter" | grep -iq "^1.$"
    if [ $counter = 18 ]
        counter=$(expr $counter + 1)
        break
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=26
while echo "$counter" | grep -iq "^2.$"
    if [ $counter = 27 ]
        counter=$(expr $counter + 1)
        if [ $counter = 28 ]
            break
        fi
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=36
while echo "$counter" | grep -iq "^3.$"
    if [ $counter = 37 ]
        counter=$(expr $counter + 1)
        if [ $counter = 38 ]
            counter=$(expr $counter + 1)
            break
        fi
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=48
while echo "$counter" | grep -iq "^4.$"
    for x in foo bar
        if [ $x = bar ]
            break
        fi
        echo $x
    done
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

echo =5=
