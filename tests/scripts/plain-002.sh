ulimit -n 999
ulimit -n
ulimit -n 888
ulimit -n
ulimit -S -c 256
ulimit -H -c 1024
ulimit -c -H
ulimit -c

echo ===1===

read a <<< hi
echo $a

read a b <<< "hello world"
echo $b $a

read c b a <<< "1 2"
echo "a=$a b=$b c=$c"

read a b <<< '1 2 3 4'
echo "a=$a b=$b"

echo ===2===

export PATH_OLD="$PATH"
export PATH=/usr/bin:/bin:/opt/homebrew/envs/somename/bin:/usr/local/bin
unpath /opt/homebrew/envs/somename/bin
echo $PATH
export PATH="$PATH_OLD"
echo ===3===
