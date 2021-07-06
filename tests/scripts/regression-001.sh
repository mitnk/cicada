export FOO=foo
echo "A${FOO}B$FOO/C${FOO}D"
echo "A$FOO/B${FOO}C$FOO/D"

# test cd/pwd on the absolute path string
mkdir -p foo1/bar1
cd foo1/bar1
cd ..
pwd | awk -F/ '{print $NF}'
cd ..
rm -rf ./foo1
echo =1=
yes | cat | cat | head -n 999999 | head -n 1
echo =2=
