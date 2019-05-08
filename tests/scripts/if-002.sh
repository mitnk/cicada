# else br returns
if echo foo | grep -iq ar
    echo found foo
else
    echo not found  # <- executed
fi
echo bye
