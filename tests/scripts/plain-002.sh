if echo hi > /dev/null
    findext 2>/dev/null || echo findext not found
fi

if echo hi > /dev/null
    ifif7 2>/dev/null || echo ifif7 not found
fi

for x in foo
    ford 2>/dev/null || echo ford not found
done

for x in foo
    doned 2>/dev/null || echo doned not found
done

counter=19
while echo "$counter" | grep -iq "^1.$"
    whiled 2>/dev/null || echo whiled not found
    counter=$(expr $counter + 1)
done
