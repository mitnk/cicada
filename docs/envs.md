# Cicada Environment Variables

You can modify them in [RC-file](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md).

## CICADA_LOG_FILE

Cicada write some logs into this file. It's raraly useful. If it not set,
there won't be any logs be written.

default: `""` (empty)

## CICADA_GITBR_PREFIX

In [prompt item](https://github.com/mitnk/cicada/blob/master/docs/prompt.md#available-prompt-items)
`$GITBR`, works as a prefix if defined.

default: `""` (empty)

## CICADA_GITBR_SUFFIX

In prompt item `$GITBR`, works as a suffix if defined.

default: `""` (empty)

## HISTORY_DELETE_DUPS

Should cicada delete duplicated history items for you?

default: `1`

## HISTORY_FILE

Specify the sqlite database file path.

default: `$XDG_DATA_HOME/cicada/history.sqlite` (if `$XDG_DATA_HOME` is set)  
default: `$HOME/.local/share/cicada/history.sqlite` (else)

## HISTORY_SIZE

How many history items should be loaded when cicada starts.

default: `99999`

## HISTORY_TABLE

Specify the table name of the history to save in.

default: `cicada_history`

## NO_EXIT_ON_CTRL_D

Do not exit cicada on `Ctrl-D`, if this env is set to `1`.

default: `0`

## Other Built-in Variables

```
$ ls file-not-exist
$ echo $?  # <-- print exit status of previous command
1

$ echo $$  # <-- print PID of current process (cicada)
2173
```
