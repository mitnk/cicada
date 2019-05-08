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
