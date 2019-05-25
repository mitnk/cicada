# NOTE: this script will print two lines into stderr,
# which will not break the tests
x=foo
if echo hi
    if [ $x = foo ]
        echo before printing continue
        continue
        echo after printing continue
    fi
    echo end of outer if
fi
echo bye

# just a comment
continue

echo bye bye
