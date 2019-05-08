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
