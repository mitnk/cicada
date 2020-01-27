# Cicada Shell Builtins

- Builtin Commands
    - [alias](#alias)
    - [bg](#bg)
    - [cd](#cd)
    - [cinfo](#cinfo)
    - [exec](#exec)
    - [exit](#exit)
    - [export](#export)
    - [fg](#fg)
    - [history](#history)
    - [jobs](#jobs)
    - [source](#source)
    - [unalias](#unalias)
    - [vox](#vox)
- [Builtin Shell Expansions](#builtin-shell-expansions)

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
$ history delete <item-id>
```

## jobs

Listing all jobs in [job control](https://github.com/mitnk/cicada/blob/master/docs/jobc.md).
See also `bg`, `fg`.

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

## unalias

Remove an alias. Usage example: `$ unalias ls`.

## vox

First, tell cicada where is your root directory of virtualenv in
[rcfile](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md).
One example:

```
export VIRTUALENV_HOME="${HOME}/.local/share/venvs"
```

Create your env with something like:

```
python3 -m venv ~/.local/share/venvs/my-project
```

Then use `vox` to enter it:

```
$ vox enter my-project
(my-project) $  # now you activated your env
```

List your envs under `$VIRTUALENV_HOME` dir:
```
$ vox ls
```

Exit (deactivate) your env:
```
(my-project) $ vox exit
$  # now you're clean
```

## Builtin Shell Expansions

### Brace Expansion

```sh
$ echo sp{el,il,al}l
spell spill spall

$ cp foo.txt{,.bak}
# equal to `cp foo.txt foo.txt.bak`

$ echo {1..5}
1 2 3 4 5

$ echo {1..5..2}
1 3 5
```

### Tilde Expansion

```
$ echo ~/foo
# equal to echo $HOME/foo
```

### Parameter Expansion

Currently only works in scripting.

```sh
$ cat foo.sh
echo "the args are: $@"
echo $3 $1 $2
echo $0

$ cicada foo.sh a b c
the args are: a b c
c a b
foo.sh
```

### Command Substitution

Command substitution allows the output of a command to replace the command
itself. Command substitution occurs when a command is enclosed as follows:

```
$(command)
```

or
```
`command`
```

### Filename Expansion

```
$ echo src/*.rs
src/build.rs src/execute.rs src/history.rs src/jobc.rs ...
```

### Special Expansions

```sh
# current session process ID
$ echo $$
26653

# last command exit status
$ echo $?
0

$ cat /etc/some-config

# last command substitution
$ sudo !!
sudo cat /etc/some-config
```
