echo hi
for xxx in foo bar baz
    echo $xxx
done

for xxx in $(echo a b)
    echo hello && echo $xxx
done

for xxx in 'args kwargs' "sh script"
    echo $xxx
done

echo bye
