# Cicada History

Shell history items are stored with sqlite database. Like bash, you can use
`Ctrl-R`, `Ctrl-P`, `Ctrl-N`, `Arrow-UP`, `Arrow-DOWN` keys to access history.

You can prevent commands from saving into history by prefixing spaces with
them.

You can modify the settings of history related value in `~/.cicadarc`. These
values on the right side are the default ones:

```
export HISTORY_FILE="$HOME/.local/share/cicada/history.sqlite"
export HISTORY_SIZE=100000
export HISTORY_TABLE="cicada_history"
```

See more on [history built-in command](https://github.com/mitnk/cicada/blob/master/docs/built-in-cmd.md#history)  
See more on [Environment Variables](https://github.com/mitnk/cicada/blob/master/docs/envs.md#history_size)  
