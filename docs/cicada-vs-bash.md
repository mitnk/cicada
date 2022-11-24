# Cicada is not Bash

While I built cicada to replace bash as a daily-use shell, cicada is not as
mature as bash. Some "complex" command lines may not work as expected. I
recommend that you have `bash` installed on your system too. Occassionally you
need to use bash as the receiver of a "copy-and-paste" command from the
internet.

For [example](https://www.gnu.org/software/gawk/manual/html_node/Quoting.html),
the following commands in will print out `single quote: <'>`.

```
$ awk 'BEGIN { print "single quote: <'"'"'>" }'
$ awk -v sq="'" 'BEGIN { print "single quote <" sq ">" }'
```

Either of them work in current cicada.

In cicada, you should avoid complex quoting things. Instead, you can write it
like this:

```
$ awk 'BEGIN { print "single quote: <\47>" }'
```

Todo: add more examples that differ from bash.
