echo "begin of script"

echo 'begin exp if'
if echo "hi foo 中文" | grep -iq foo

  echo "looks nice"
  echo else

else

    echo 'not found'

fi

alias ls='ls -lh'
echo bye
