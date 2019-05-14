plain_counter=35
echo hi
echo mid > /dev/null

echo hello \
       world

echo foo\
bar

echo hello world \
    | sed 's/hello/hi/' \
    | wc \
    | sed -e 's/  */ /g' \
          -e 's/^  *//'

echo bye "counter $plain_counter"
