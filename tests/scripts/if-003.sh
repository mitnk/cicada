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
