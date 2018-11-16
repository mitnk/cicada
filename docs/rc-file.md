# RC File

Cicada use RC file: "~/.cicadarc". Currently only support ENVs and aliases:

```
# A sample of RC file
export RUST_BACKTRACE='full'
export COPYFILE_DISABLE=1
export PATH="/usr/local/bin:$PATH"

alias ls="ls -G"
alias ll="ls -lh"

# you can also include extra rc files,
include .cicadarc_local
# or with full path
include /etc/cicadarc
```
