echo begin if exp

    if echo "foo 中文" | grep -qi foo

echo 'we found it'

    else

echo not found

    fi

echo "after fi"
echo the end

