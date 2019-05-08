while echo $xxx | grep -iqv "final"
    echo "xxx begins with $xxx"
    for xxx in wip final
        echo xxx now is $xxx
    done
done
