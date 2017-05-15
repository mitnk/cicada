# Cicada Unix Shell

[![Travis Build Status](https://api.travis-ci.org/mitnk/cicada.svg?branch=master)](https://travis-ci.org/mitnk/cicada)
[![Latest Version](https://img.shields.io/crates/v/cicada.svg)](https://crates.io/crates/cicada)

Cicada is a simple, semi-toy Unix shell written in Rust. And I use it as
my default login shell.

## Features so far

### run programs and pipelines

```
$ ls | head -n3
Desktop
Documents
Downloads

$ echo foo bar | awk -F " " '{print $2, $1}'
bar foo
```

### with redirections

```
$ ls file-not-exist 2>&1 | wc > e.txt
$ cat e.txt
       1       7      46
```

### command substitution

```
$ ls -l `which sh`
-r-xr-xr-x  1 root  wheel  630464 Mar 23 07:57 /bin/sh
```

### run multiple commands (with logical)

```
$ echo foo; echo bar
foo
bar

$ echo foo && echo bar
foo
bar

$ echo foo || echo bar
foo
```

### shell expansions

```
$ echo sp{el,il,al}l
spell spill spall

$ echo $SHELL
/usr/local/bin/cicada

$ echo *
Cargo.lock Cargo.toml LICENSE Makefile README.md src target
```

### do math arithmetic directly in the shell!

```
$ 1 + 2 * 3 - 4
3
$ (1 + 2) * (3 - 4) / 8.0
-0.375
```

## RC File

Cicada use RC file: "~/.cicadarc". Currently only support ENVs and aliases:

```
# A sample of RC file
export RUST_BACKTRACE='full'
export COPYFILE_DISABLE=1
export PATH="/usr/local/bin:$PATH"

alias ls="ls -G"
alias ll="ls -lh"
```

## Completions

Path completion is available out of box. In addition to this, cicada also
supports simplifed customized completion using YAML file. Put your completion
files under `~/.cicada/completers/`. The completion files look like this:

```
$ ls ~/.cicada/completers/
brew.yaml git.yaml  pip.yaml  vox.yaml

$ cat ~/.cicada/completers/brew.yaml
- doctor
- info
- install
- list
- search
- uninstall
- update
- upgrade

$ brew u<Tab><Tab>
uninstall  update  upgrade
```

Currently, cicada supports maximum 2 level completion:

```
$ cat ~/.cicada/completers/pip.yaml
- install:
    - --force-reinstall
    - -U
    - --upgrade
    - -r
    - --requirement
    - --user
- download
- uninstall
- freeze
- list
- show
- check
- search:
    - --no-cache-dir
    - --timeout
- wheel
- hash
- completion
- help

$ pip ins<Tab>
$ pip install

$ pip install --re<Tab>
$ pip install --requirement
```

## History

Shell history items are stored with sqlite database. Like bash, you can use
`Ctrl-R`, `Ctrl-P`, `Ctrl-N`, `Arrow-UP`, `Arrow-DOWN` keys to access history.

You can modify the settings of history related value in `~/.cicadarc`. These
values on the right side are the default ones.

```
export HISTORY_FILE="~/.local/share/cicada/history.sqlite"
export HISTORY_SIZE=9999
export HISTORY_TABLE="cicada_history"
```

## Install Cicada

Note: [Rust environment](https://rustup.rs/) is needed for installation.

You can try `cicada` out without installing it by checking out the repository
and run `cargo run` in its root directory.

```
$ git clone https://github.com/mitnk/cicada
$ cd cicada
$ cargo run
```

### install from code repository

If you've checked out the cicada repository, you can do this:

```
$ make install
```

This will install `cicada` under your `/usr/local/bin`. Use `sudo` if needed.

### install via cargo crates

```
$ cargo install -f cicada
```

This will install cicada into `~/.cargo/bin/`.

### Set cicada as your login shell

Appending `/usr/local/bin/cicada` into your `/etc/shells`, then run
```
$ chsh -s /usr/local/bin/cicada
```

## To do list

- job controls (`Ctrl-Z`, `fg`, `bg` etc)
- and less...

## Won't do list

- functions
- Windows support
- and more...
