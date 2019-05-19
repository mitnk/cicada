# Scripting in Cicada

The goal of cicada is to be a useful daily-use shell and replace Bash.
It does not intend to compete with shells like zsh, fish, etc. Cicada keeps
[KISS Principle](https://en.wikipedia.org/wiki/KISS_principle) in mind.
For scripting, cicada won't introduce a full featured scripting
language as bash did. For complex scripting job, I would recommend you
to use bash (and call them with `$ bash xxx.sh` in cicada), or dynamic
scripting languages like Python. Scripting with cicada should only be used
in simple cases.

- [Introduction](#introduction)
- [If Statements](#if-statements)
- [For Statements](#for-statements)
- [While Statements](#while-statements)
- [Using Builtins](#using-builtins)
- [Functions](#functions)

## Introduction

Firstly, cicada supports run commands (or pipes) line by line from a file:

File content of `~/hello.sh`:
```sh
#!/usr/local/bin/cicada
echo hello scripting
# spliting command into multiple lines
echo hi \
      there
echo "the args are: $@"
echo $3 $1 $2
date
echo bye
```

We can make this file as executable with:
```
$ chmod +x ~/hello.sh
```

Then there are two methods to run it:

**a) Run it directly**
```
$ ~/hello.sh foo bar baz
```

**b) Pass it to cicada**
```
$ cicada ~/hello.sh foo bar baz
```

Either way, the output looks like this:

```
hello scripting
hi there
runing /home/mitnk/hello.sh with args: foo bar baz
baz foo bar
Sat Apr 27 17:14:36 CST 2019
bye
```

## If Statements

In every `if` statement, each test conditions are checked one by one,
and run cmds in first True condition. True conditions means the commands
that exit with `status 0`.

```sh
if echo foo | grep -iq o
    echo found foo
fi

if echo foo | grep -iq bar
    echo found bar
else
    echo not found bar
fi

if echo foo | grep -iq a
    echo found a
else if echo foo | grep -iq b
    echo found b
else
    echo no a and no b
fi
```

The output of above script is:
```
found foo
not found bar
no a and no b
```

### Use test `[` command

The [test](https://linux.die.net/man/1/test) command `[` is a convenient tool
to use in `if` and `while` statements.

```
foo=35
if [ $foo -gt 10 ]
    echo "foo is great than 10"
else
    echo "foo is less than 10"
fi

if [ $(uname -s) = 'Darwin' ]
    echo "you're in Mac OS"
fi
```

For for details, please check out `[ --help` in your shell.

**Note:** Compare strings with `[ $str1 > $str2 ]` is not supported in cicada.
The `>` would be treated as output redirections.

## For Statements

In cicada, `for` statement loop the space splitted strings. In each iteration,
the string is assigned to the variable, and run commands with this just
available variable.

```sh
for var in foo bar baz
    echo $var
done

for var2 in $(echo a b)
    echo hello && echo $var2
done

for var3 in 'args kwargs' "sh script"
    echo $var3
done

for f in src/builtins/ex*.rs
    echo source file $f
done

for x in {1..10..2}
    echo "x = $x"
done
```

The output of above script is:
```
foo
bar
baz

hello
a
hello
b

args kwargs
sh script

source file src/builtins/exec.rs
source file src/builtins/exit.rs
source file src/builtins/export.rs

x = 1
x = 3
x = 5
x = 7
x = 9
```

## While Statements

In `while` statements, the command body will be run whenever the test branch
is still true.

```sh
counter=17
while echo "$counter" | grep -iq "^1.$"
    echo "counter = $counter"
    counter=$(expr $counter + 1)
done
```

The output is:
```
counter = 17
counter = 18
counter = 19
```

## Combine If, For, While Together

As expected, you can combine/nested the above statements together.

One example is set `ls` aliases in `~/.cicadarc`:

```sh
if which exa > /dev/null
    alias ls='exa'
    alias ll='exa -lh --time-style=long-iso'
else
    if uname -s | grep -iq 'darwin'
        alias ls='ls -G'
        alias ll='ls -Glh'
    else
        alias ls='ls --color=auto'
        alias ll='ls -lh --color=auto'
    fi
fi
```

Another example:

```sh
counter=17
if echo foo | grep -q oo
    while echo "$counter" | grep -iq "^1.$"
        echo "counter = $counter"
        counter=$(expr $counter + 1)
    done
fi

if echo foo | grep -q oo
    if echo bar | grep -q oo
        echo found oo
    else
        while echo "$counter" | grep -iq "^2[0-2]$"
            echo "counter = $counter"
            counter=$(expr $counter + 1)
        done
    fi
fi
```

The output is:
```
counter = 17
counter = 18
counter = 19

counter = 20
counter = 21
counter = 22
```

## The source Builtin

> See also [the source builtin](https://github.com/mitnk/cicada/blob/master/docs/builtins.md#source).

Command like `$ cicada foo.sh` would create a new session and run the commands
of file `foo.sh`. If you want to run them in current shell session, you
can run it with `$ source foo.sh`.

## Using Builtins

In scripts, you could also use cicada's
[builtins](https://github.com/mitnk/cicada/blob/master/docs/builtins.md).
For example, you can include extra RC configs with `source` at the end of
`~/.cicadarc` file:
([RC file](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md)
itself is also a valid cicada script).

```
# my cicada rc file: ~/.cicadarc
alias ll='ls -lh'

# other settings
...

# include some extra settings for this host only:
source ~/.cicadarc_local
```

## Functions

A function is defined like this:

```txt
function <function-name>() {
    <function-body>
}
```

**NOTE:** The `function <function-name>() {` and `}` part must be in its own
line. The `()` part is optional.

One example:

```
$ cat bar.sh
function foo-bar() {
    echo hi
    echo $0
    echo $1 $2
    echo bye
}
```

After define it, you can call it in the same file (`bar.sh`) like this:
```
foo-bar arg1 arg2
```

Another way is you can `source` this file and run this function in the shell:
```
$ source bar.sh
$ foo-bar arg1 arg2 arg3
```

The output would be:
```
hi
foo-bar
arg1 arg2
bye
```
