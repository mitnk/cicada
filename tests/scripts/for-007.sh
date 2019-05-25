for x in foo bar baz
    echo $x
    break
done

echo first round
for x in 1 2 3
    for y in a b c
        if [ "$x" = "2" ]
            break
        fi
        echo "$y$x"
    done
done

echo second round
for x in 1 2 3
    for y in a b c
        if [ "$y" = "b" ]
            break
        fi
        echo "$y$x"
    done
done
