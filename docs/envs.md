# Cicada Environment Variables

You can modify them in `~/.cicadarc` file.

## HISTORY_SIZE

How many history items should be loaded when cicada starts.

default: `999`

## HISTORY_DELETE_DUPS

Should cicada delete duplicated history items for you?

default: `1`

## HISTORY_FILE

Specify the sqlite database file path.

default: `$XDG_DATA_HOME/cicada/history.sqlite` (if `$XDG_DATA_HOME` is set)  
default: `$HOME/.local/share/cicada/history.sqlite` (else)

## HISTORY_TABLE

Specify the table name of the history to save in.

default: `cicada_history`
