# Cicada Shell Builtins

- Builtin Commands
    - [alias](#user-content-alias)
    - [bg](#user-content-bg)
    - [cd](#user-content-cd)
    - [check](#user-content-check)
    - [cinfo](#user-content-cinfo)
    - [exec](#user-content-exec)
    - [exit](#user-content-exit)
    - [export](#user-content-export)
    - [fg](#user-content-fg)
    - [history](#user-content-history)
    - [jobs](#user-content-jobs)
    - [read](#user-content-read)
    - [set](#user-content-set)
    - [source](#user-content-source)
    - [ulimit](#user-content-ulimit)
    - [unalias](#user-content-unalias)
    - [unpath](#user-content-unpath)
    - [unset](#user-content-unset)
    - [vox](#user-content-vox)

## alias

Aliases allow a string to be substituted for a word when it is used as
the first word of a simple command.

Aliases are created and listed with the alias command, and removed with
the `unalias` command.

```
alias [name][=value]
```

Without arguments, alias prints the list of aliases on the standard output
in a form that allows them to be reused as input. If arguments are supplied,
an alias is defined for each name whose value is given.
If no value is given, the name and value of the alias is printed.

## bg

Make stopped job runing in background. See also `fg`, `jobs`.

## cd

Change your current work directory.

## check

Check what a command name refers to: alias, builtin, or external command.

```
check <command>
```

Examples:
```
$ check file
/usr/bin/file: Mach-O universal binary

$ check ll
alias ll="ls -lh"
/bin/ls: Mach-O universal binary

$ check k
alias k="kubectl"
/opt/homebrew/bin/kubectl: Mach-O 64-bit executable
realpath: /opt/homebrew/Cellar/kubernetes-cli/1.32.1/bin/kubectl
```

## cinfo

Print information of cicada and OS.

## exec

If command is specified, it replaces the shell. No new process is created.
The arguments become the arguments to command.

## exit

Exit the current progress (the shell). Can exit with an extra code like:
`exit 2`.

## export

Change environment variables for current session. You can also use `export` in
[RC-file](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md).

Examples:
```
$ export PATH="/usr/local/bin:$PATH:$HOME/.cargo/bin"
$ export RUST_BACKTRACE=full
$ export PYTHONPATH=.
```

## fg

Bring background job into foreground. See also `bg`, `jobs`.

## history

### List your recent history

```
$ history
1: touch docs/envs.md
2: mvim docs/envs.md
3: find . -name '*.bk' | xargs rm
4: find . -name '*.bk'
```

### Search history items (use `%` from SQL to match "anything")

```
$ history curl
1: curl -x http://127.0.0.1:1080 https://hugo.wang/http/ip/
2: curl -I https://twitter.com/

$ history 'curl%hugo'
1: curl -x http://127.0.0.1:1080 https://hugo.wang/http/ip/
```

### More features

```
$ history --help

USAGE:
    history [FLAGS] [OPTIONS] [PATTERN]

FLAGS:
    -a, --asc          Search old items first
    -h, --help         Prints help information
    -n, --no-id        Do not show ROWID
    -o, --only-id      Only show ROWID
    -p, --pwd          For current directory only
    -s, --session      For current session only
    -d, --show-date    Show date
    -V, --version      Prints version information

OPTIONS:
    -l, --limit <limit>     [default: 20]

ARGS:
    <PATTERN>    You can use % to match anything [default: ]
```

### Delete history items

```
$ history delete <item-id> [item-id, ...]
```

### Add history items

```
$ history add '<the command input>'
```

## jobs

Listing all jobs in [job control](https://github.com/mitnk/cicada/blob/master/docs/jobc.md).
See also `bg`, `fg`.

## read

Read a line from the standard input and split it into fields.

```
read [name ...]
```

Reads a single line from the standard input. The line is split into fields as
with word splitting, and the first word is assigned to the first NAME, the
second word to the second NAME, and so on, with any leftover words assigned to
the last NAME.  Only the characters found in `$IFS` are recognized as word
delimiters.

If no NAMEs are supplied, the line read is stored in the `REPLY` variable.

The following example prints `bar foo`:

```sh
$ read v1 v2
foo bar<hit ENTER>
$ echo $v2 $1
```

and the following example prints `5 3 1`:
```sh
$ IFS=:@ read a b c
1 3 5<hit ENTER>
$ echo $c $b $a
```

## set

(in BETA) Set shell options. Currently ony support `set -e`, same effects
as Bash.

## source

Read and execute commands from the `filename` argument in the current shell
context. It stops at first command with non-zero return status.

```
source filename
```

If filename does not contain a slash, the PATH variable is used to
find `filename`. The current directory is searched if `filename` is not
found in `$PATH`. If any arguments are supplied, they become the positional
parameters when filename is executed. The return status is the exit status
of the last command executed, or zero if no commands are executed. If
`filename` is not found, or cannot be read, the return status is non-zero.
Like in Bash, **this builtin is equivalent to `.` (a period)**.

## ulimit

> See `ulimit --help` for more usage.

Show shell limits:
```
$ ulimit -a
open files		256
core file size		0
```

Change limit of open files to 10240
```
$ ulimit -n 1024

$ ulimit -n  # check the new value
1024
```

Set hard limits instead of the default soft limits
```
$ ulimit -c -H  # check the new value of the hard limit
unlimited

$ ulimit -c 65535

$ ulimit -c -H  # check the new value of the hard limit
65535
```

Currently, only `-n` (open files) and `-c` (core file size) is supported.

## unalias

Remove an alias. Usage example: `$ unalias ls`.

## unpath

Remove one item from the system variable `PATH`. Example:
```
$ echo $PATH
/usr/bin:/bin:/opt/homebrew/envs/somename/bin:/usr/local/bin
$ unpath /opt/homebrew/envs/somename/bin
$ echo $PATH
/usr/bin:/bin:/usr/local/bin
```

## unset

Delete a variable or function by its name.

```sh
$ export FOO="some value"
# prints "some value"
$ echo $FOO

$ unset FOO
# prints ""
$ echo $FOO
```

## vox

First, tell cicada where is your root directory of virtualenv in
[rcfile](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md).
One example:

```
export VIRTUALENV_HOME="${HOME}/.local/share/venvs"
```

Create your env with python:

```
$ python3 -m venv ~/.local/share/venvs/my-project
```

or create it with `vox`:
```
$ vox create my-project
```
this will create `my-project` venv under `$VIRTUALENV_HOME`.

Then use `vox` to enter it:

```
$ vox enter my-project
(my-project) $  # now you activated your env
```

List your envs under `$VIRTUALENV_HOME` directory:
```
$ vox ls
```

Exit (deactivate) your env:
```
(my-project) $ vox exit
$  # now you're clean
```
