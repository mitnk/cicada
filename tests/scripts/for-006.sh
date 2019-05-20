echo zero round
for x in 1 2 3
    for y in a b c
        echo "$y$x"
    done
done

echo first round
for x in 1 2 3
    for y in a b c
        if [ "$x" = "2" ]
            continue
        fi
        echo "$y$x"
    done
done

echo second round
for x in 1 2 3
    for y in a b c
        if [ "$y" = "b" ]
            continue
        fi
        echo "$y$x"
    done
done
