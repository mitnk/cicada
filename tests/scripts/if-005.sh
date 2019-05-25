# NOTE: this script will print two lines into stderr,
# which will not break the tests
x=foo
if echo hi
    if [ $x = foo ]
        echo before printing break
        break
        echo after printing break
    fi
    echo end of outer if
fi
echo bye

# just a comment
break

echo bye bye
