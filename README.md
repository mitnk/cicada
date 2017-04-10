# Cicada

A simple, semi-toy Unix shell written in Rust.


## Install Cicada Shell

Note: [Rust environment](https://rustup.rs/) is needed for installation

You can try `cicada` out without installing it by checking out the repository
and run `cargo run` in its root directory.

### install cicada

```
$ make install
```

This will install `cicada` under your `/usr/local/bin`. Use `sudo` if needed.

### Set cicada as your login shell

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

# specify the history file,
# its default path is "~/.local/share/cicada/history.sqlite"
export HISTORY_FILE=/Users/mitnk/.local/share/xonsh/xonsh-history.sqlite

alias ls="ls -G"
alias ll="ls -lh"
```

## Completions

see doc (to add)


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
