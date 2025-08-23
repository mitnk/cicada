#!/bin/sh
set -e
CICADA="./target/debug/cicada"

if [ ! -f $CICADA ]; then
    echo "cicada binary not found: $CICADA"
    echo "please build it out with cargo build first"
    exit 1
fi

DIR_TEST="$( cd "$(dirname "$0")" ; pwd -P )"
TMP_FILE="${DIR_TEST}/tmpfile-test-cicada-scripts.out"

for src in ${DIR_TEST}/scripts/*.sh; do
    echo Runing $CICADA $src
    $CICADA $src > "$TMP_FILE"
    if diff -u "${src}.out" $TMP_FILE; then
        echo OK.
    else
        echo Failed.
        exit 1
    fi
done

rm -f $TMP_FILE
