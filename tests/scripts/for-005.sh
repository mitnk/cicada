for x in foo bar
        continue
done

for x in 1 2 3
    if [ "$x" = "2" ]
        continue
    fi
    echo $x
done

for x in 4 5 6
    if [ "$x" = "4" ]
        if [ "$x" = "4" ]
            continue
        fi
    fi
    echo $x
done

for x in 8 9 10
    if [ "$x" = "10" ]
        if [ "$x" = "10" ]
            continue
        fi
    fi
    echo $x
done

for x in a b
    if [ "$x" = "c" ]
        continue
    fi
    echo $x
done
