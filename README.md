# Cicada

[![Travis Build Status](https://api.travis-ci.org/mitnk/cicada.svg?branch=master)](https://travis-ci.org/mitnk/cicada)
[![Latest Version](https://img.shields.io/crates/v/cicada.svg)](https://crates.io/crates/cicada)

A simple, semi-toy Unix shell written in Rust.


## Install Cicada Shell

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

## Set cicada as your login shell

Appending `/usr/local/bin/cicada` into your `/etc/shells`, then run
```
$ chsh -s /usr/local/bin/cicada
```

## Usage (Features so far)

```
# run programs
$ ls
Desktop
Documents
Downloads
Dropbox
Movies
Music

# with pipeline
$ man awk | awk -F "[ ,.\"]+" '{for(i=1;i<=NF;i++)A[$i]++}END{for(k in A)print k, A[k]}' | sort -k2nr | head -n8
the 70
of 40
a 27
is 27
and 24
are 21
in 21
to 21

# with redirections
$ ls file-not-exist 2>&1 | wc > e.txt
$ cat e.txt
       1       7      46

# do math arithmetic
$ 1 + 2 * 3 - 4
3
$ (1 + 2) * (3 - 4) / 8.0
-0.375
```

## RC File

Cicada use RC file: "~/.cicadarc". Currently only support ENVs and aliases:

```
# A sample RC file
export RUST_BACKTRACE='full'
export LESS="-R"
export COPYFILE_DISABLE=1

export PATH="/usr/local/bin:$PATH"

# specify the history file,
# its default path is "~/.local/share/cicada/history.sqlite"
export HISTORY_FILE=/Users/mitnk/.local/share/xonsh/xonsh-history.sqlite

alias ls="ls -G"
alias ll="ls -lh"
```

## Completions

Cicada currently only support simplifed completion. It uses YAML files
to define two level completion. Put your completion files under `~/.cicada/completers/`.
And one completion file is like this:

```
$ ls ~/.cicada/completers/
brew.yaml git.yaml  pip.yaml  vox.yaml

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
```

After define this file, you can complete `pip` with `$ pip ins<Tab>` to get
`$ pip install`, and `$ pip install --re<Tab>` to get
`$ pip install --requirement`.

```
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


## To do list

- [Special characters](http://tldp.org/LDP/abs/html/special-chars.html)
- [Shell](http://tldp.org/LDP/Bash-Beginners-Guide/html/sect_03_04.html) [expansion](http://wiki.bash-hackers.org/syntax/expansion/globs)
- job controls (`Ctrl-Z`, `fg`, `bg` etc)
- and less...


## Won't do list

- functions
- Windows support
- and more...


## Related projects

- [xonsh](https://github.com/xonsh/xonsh) - A python-powered, cross-platform,
Unix-gazing shell.
