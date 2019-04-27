# Scripting in Cicada

The goal of cicada is to be a useful daily-use shell and replace Bash. But
it does not intend to compete with shells like zsh, fish, etc. It keep
[KISS Principle](https://en.wikipedia.org/wiki/KISS_principle) in mind.
For scripting, cicada won't introduce a full featured scripting
language as bash did. For complex scripting job, I would recommend you
to use bash (and call them with `$ bash xxx.sh` in cicada), or dynamic
scripting languages like Python. Scripting with cicada should be only used
in simple cases.

## Introduction

Currently its only support put commands (or pipes) line by line into a file:

File content of `~/hello.sh`:
```
#!/usr/local/bin/cicada
echo hello scripting
echo "the args are: $@"
echo $3 $1 $2
date
echo bye
```

We can make this file as executable with:
```
$ chmod +x ~/hello.sh
```

Then there are two method to run it:

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
runing /home/mitnk/hello.sh with args: foo bar baz
baz foo bar
Sat Apr 27 17:14:36 CST 2019
bye
```

## The source Builtin

> See also [the source builtin](https://github.com/mitnk/cicada/blob/master/docs/built-in-cmd.md#source).

Command like `$ cicada foo.sh` would create a new session and run the commands
in file `foo.sh`. If you want to run then in current shell session, you
can run it with `$ source foo.sh`.

## Using Builtins

In scripts, you could also use cicada's
[builtins](https://github.com/mitnk/cicada/blob/master/docs/built-in-cmd.md).
For example, you can include extra RC configs at the end of `~/.cicadarc` file:
([RC file](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md)
itself is also a valid cicada script).

```
# my cicada rc file: ~/.cicadarc
alias ll='ls -lh'

# other settings
...

# including some extra settings for this host only:
source ~/.cicadarc_local
```

## Functions is not Supported Yet

Supporing functions in cicada is in the to do list. But as said in beginning
of this doc, it could be also a simplified thing.
