# Cicada History

Shell history items are stored with sqlite database. You can find the file
location with `cinfo` command. Like bash, you can use `Ctrl-R`, `Ctrl-P`,
`Ctrl-N`, `Arrow-UP`, `Arrow-DOWN` keys to access history.

Recommend that in your `~/.inputrc`, you have:

```
"\e[1;9D": backward-word
"\e[1;9C": forward-word
"\e[A": history-search-backward
"\e[B": history-search-forward
```

> With above inputrc setup, you can type a part (prefix) of history, and
> then hit `<Arrow-Up>` to retrive items start with this prefix:
> ```
> $ echo 1.23
> $ echo foo bar
> $ echo 1<Arrow-UP>  # will get you `echo 1.23`
> ```

## Prefixing spaces will suppress history to be saved

You can prevent commands from saving into history by prefixing spaces with
them.

```
$ echo foo bar
$ echo 123
$  echo secret 235
  ^ prefix command with a space to make command skip saving to history.
$ history
2: echo 123
1: echo foo bar
```

## History Environment Variables

There are several environment variables on history.

```
export HISTORY_FILE="$HOME/.local/share/cicada/history.sqlite"
export HISTORY_SIZE=99999
export HISTORY_TABLE="cicada_history"
```

You can modify the settings of history related value in
[RC-file](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md).

## History is Immutable

In Bash, you can edit history items. However in cicada, the history items
are immutable. You cannot edit them. But you could delete them.

## The history builtin command

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

See more details here: [history built-in command](https://github.com/mitnk/cicada/blob/master/docs/builtins.md#user-content-history)

## Others

See more on [Environment Variables](https://github.com/mitnk/cicada/blob/master/docs/envs.md#user-content-history_size)
