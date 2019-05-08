echo hi
counter=17
while echo "$counter" | grep -iq "^1.$"
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done
echo bye
