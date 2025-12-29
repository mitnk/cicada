# Cicada Environment Variables

You can modify them in [RC-file](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md).

## CICADA_CMD_WRAPPERS

A colon-separated list of additional command wrappers. Command wrappers are
commands that execute another command (like `sudo`, `nohup`, `xargs`). Cicada
uses this for syntax highlighting and tab completion.

Built-in wrappers: `sudo`, `xargs`, `nohup`, `nice`, `ionice`, `time`,
`timeout`, `env`, `exec`, `caffeinate`, `command`, `builtin`, `which`.

Example:
```
export CICADA_CMD_WRAPPERS=doas:strace:gdb
```

default: `""` (empty, only built-in wrappers)

## CICADA_ENABLE_SIG_HANDLER

Cicada will install a self-defined signal handler if its value set to `1`.  But
it may cause cicada crash in some cases, as it's [not safe
yet](https://github.com/rust-lang/rfcs/issues/1368) in rust to use signal
handlers.

default: `""` (empty, disabled)

When cicada is used as container [CMD](https://docs.docker.com/engine/reference/builder/#cmd)
instruction (PID 1), this env should be set to `1`, so that zombie processes
are reapped correctly.

## CICADA_LOG_FILE

Cicada write some logs into this file. It's raraly useful. If it not set,
there won't be any logs be written.

default: `""` (empty)

## CICADA_GITBR_MAX_LEN

Cicada make git branch name shorter based on this value when showing it in
prompt.

default: 32

## CICADA_GITBR_PREFIX

In [prompt item](https://github.com/mitnk/cicada/blob/master/docs/prompt.md#user-content-available-prompt-items)
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
