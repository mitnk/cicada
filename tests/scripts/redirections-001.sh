# no stdout
echo hi1 > /dev/null

# stdout: 0
echo hi2 1>&2 | wc -l | awk '{print $1}'

# no stdout all these files do not exists
ls -l file-1.txt 2>/dev/null | cat

# stdout: file-2.txt
ls -l file-2.txt 2>&1 | grep -o '\<file...txt\>'

# stdout: 0
ls -l file-3.txt 2>&1 | cat 1>&2 | wc | awk '{print $1}'
echo ==1==
