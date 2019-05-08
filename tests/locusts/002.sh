echo begin exp while
while cat /tmp/foo.txt | grep -iq foo
    sleep 1
    echo "looks nice"
done

echo bye
