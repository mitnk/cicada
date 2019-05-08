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
