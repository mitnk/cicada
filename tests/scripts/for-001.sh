echo hi
for var in foo bar baz
    echo $var
done

for var2 in $(echo a b)
    echo hello && echo $var2
done

for var3 in 'args kwargs' "sh script"
    echo $var3
done

for var4 in src/builtins/ex*.rs
    echo source file $var4
done

for c in ab , ' ' xy
    echo "append '$c' into file"
done

echo bye
