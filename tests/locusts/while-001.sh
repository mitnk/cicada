echo while begins
while cat /tmp/foo.txt | grep -qi foo

    sleep 1
    echo foo is good in while
done
echo the end
