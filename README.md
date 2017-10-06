# Cicada Unix Shell

[![Travis Build Status](https://api.travis-ci.org/mitnk/cicada.svg?branch=master)](https://travis-ci.org/mitnk/cicada)
[![Latest Version](https://img.shields.io/crates/v/cicada.svg)](https://crates.io/crates/cicada)

Cicada is a simple Unix shell written in Rust. It's ready for daily use.

## Documents

- [Environment Variables](https://github.com/mitnk/cicada/tree/master/docs/envs.md)
- [Built-in Commands](https://github.com/mitnk/cicada/tree/master/docs/built-in-cmd.md)
- [Completion](https://github.com/mitnk/cicada/tree/master/docs/completion.md)
- [RC File](https://github.com/mitnk/cicada/tree/master/docs/rc-file.md)
- [History](https://github.com/mitnk/cicada/tree/master/docs/history.md)

## Features

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

## Install Cicada

### Install via Pre-built Binaries

First download the latest right binary for your system from
[Release Page](https://github.com/mitnk/cicada/releases).

Move it to right place and add runable permisson:

```
# on Mac
$ mv cicada-mac-v0.6.5 /usr/local/bin/cicada

# on Linux
$ mv cicada-linux64-v0.6.5 /usr/local/bin/cicada

$ chmod +x /usr/local/bin/cicada
```

Then you can try it by run `cicada` in your shell.

### Install Cicada via Source

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

## Cicada is also a library

Read APIs here: [https://docs.rs/crate/cicada/0.7.0](https://docs.rs/crate/cicada/0.7.0)

## FAQs

### Why another shell?

- for fun
- to learn Rust
- have a shell that can customize a bit for my own needs

I think it's fair to say cicada is just a toy.

### Compare to bash?

Bash is where most people come from and familiar with. So cicada is trying to support common cases that bash supports. Cicada will only be a "subset" of bash. If bash is a steamship, cicada is just a boat.

### Compare to ion?

[Ion](https://github.com/redox-os/ion) is a modern system shell that is also written in Rust. It's more mature as a general shell. Ion is to Rust what [xonsh](http://xon.sh) to Python, which supports following stuff:
```
$ let string = "one two three"
$ echo $string[0]
o
$ echo $string[..3]
one
```
While cicada do not and will not support these features.

### Why functions support is in won't do list?

I don't think i have interests or energy to add (bash) functions support or (bash) shell scripting ability. If you're a heavy function/scripting user, cicada may not be your tool. If you found cicada useful, you can always add your things based on it.

As far as I can see, cicada will not introduce such complex things, and will not be another zsh/fish.

### Will cicada be POSIX-compatible?

As the above anwsers hints, while cicada is trying to be POSIX, but it will not be a fully POSIX shell. However, If any command pattern is common and cicada is missing, we can add it.

### Will my bash/zsh scripts continue work in cicada?

It depends. If the script is only doing external things, like an configure/installation script, you can still run it. You can invoke scripts with `$ ./my-script.sh` as long as it have "#!/bin/bash" stuff on the top. Or you can always run them as: `$ bash my-script.sh`.

Cicada does not recognize these scripts itself. You cannot integrate these shell scripts/functions in RC files to initiate cicada shell.

### Windows support?

Cicada is a Unix shell.


## To do list

- job controls (`Ctrl-Z`, `fg`, `bg` etc)
- and less...

## Won't do list

- functions
- and more...
