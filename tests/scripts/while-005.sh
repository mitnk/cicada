counter=17
while echo "$counter" | grep -iq "^1.$"
    break
done

while echo "$counter" | grep -iq "^1.$"
    if [ $counter = 18 ]
        counter=$(expr $counter + 1)
        break
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=26
while echo "$counter" | grep -iq "^2.$"
    if [ $counter = 27 ]
        counter=$(expr $counter + 1)
        if [ $counter = 28 ]
            break
        fi
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=36
while echo "$counter" | grep -iq "^3.$"
    if [ $counter = 37 ]
        counter=$(expr $counter + 1)
        if [ $counter = 38 ]
            counter=$(expr $counter + 1)
            break
        fi
    fi
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done

counter=48
while echo "$counter" | grep -iq "^4.$"
    for x in foo bar
        if [ $x = bar ]
            break
        fi
        echo $x
    done
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done
