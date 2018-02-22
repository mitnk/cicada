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

## Cicada is also a library (BETA)

Read APIs here: [https://docs.rs/cicada/0.8.0/cicada/](https://docs.rs/cicada/0.8.0/cicada/)

## Install Cicada

Please refer to [docs/install.md](https://github.com/mitnk/cicada/blob/master/docs/install.md).

## FAQs

- [Why another shell?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#why-another-shell)
- [Compare to bash?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#compare-to-bash)
- [Compare to ion?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#compare-to-ion)
- [Why functions support is in won't do list?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#why-functions-support-is-in-wont-do-list)
- [Will cicada be POSIX-compatible?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#will-cicada-be-posix-compatible)
- [Will my bash/zsh scripts continue work in cicada?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#will-my-bashzsh-scripts-continue-work-in-cicada)
- [Windows support?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#windows-support)

## To do list

- job controls (`Ctrl-Z`, `fg`, `bg` etc)
- and less...

## Won't do list

- functions
- and more...
