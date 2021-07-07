# Cicada Unix Shell

[![Latest Version](https://img.shields.io/crates/v/cicada.svg)](https://crates.io/crates/cicada)

Cicada is a simple Unix shell written in Rust.

## Documents

- [Install cicada](https://github.com/mitnk/cicada/blob/master/docs/install.md)
- [Environment Variables](https://github.com/mitnk/cicada/tree/master/docs/envs.md)
- [Cicada Builtins](https://github.com/mitnk/cicada/tree/master/docs/builtins.md)
- [Completion](https://github.com/mitnk/cicada/tree/master/docs/completion.md)
- [RC File](https://github.com/mitnk/cicada/tree/master/docs/rc-file.md)
- [History](https://github.com/mitnk/cicada/tree/master/docs/history.md)
- [Job Control](https://github.com/mitnk/cicada/tree/master/docs/jobc.md)
- [Customize Prompt](https://github.com/mitnk/cicada/tree/master/docs/prompt.md)
- [Scripting](https://github.com/mitnk/cicada/tree/master/docs/scripting.md)

## Try out cicada with Docker

```
$ docker pull mitnk/cicada
$ docker run --rm -it mitnk/cicada
(in-cicada) $ cinfo
```

## Features

### Run programs and pipelines

```
$ ls | head -n3
Desktop
Documents
Downloads

$ echo foo,bar | awk -F "," '{print $2, $1}'
bar foo
```

### With redirections

```
$ ls file-not-exist 2>&1 | wc > e.txt
$ cat e.txt
       1       7      46
```

### Command substitution

```
$ ls -l `which sh`
-r-xr-xr-x  1 root  wheel  618512 Oct 26  2017 /bin/sh

$ echo "Time is $(date)."
Time is Sun Sep  2 12:04:13 CST 2018.
```

### Run multiple commands (with logical)

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

### Math arithmetic directly in the shell!

```
$ 1 + 2 * 3 - 4
3
$ (1 + 2) * (3 - 4) / 8.0
-0.375
$ 2 ^ 31
2147483648
```

## Cicada is also a library (BETA)

Read APIs here: [https://docs.rs/cicada/](https://docs.rs/cicada/).

## FAQs

- [Why another shell?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#user-content-why-another-shell)
- [Compare to bash/zsh/etc?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#user-content-compare-to-other-shells)
- [Is cicada POSIX-compatible?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#user-content-is-cicada-posix-compatible)
- [Will my bash/zsh scripts continue work in cicada?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#user-content-will-my-bashzsh-scripts-continue-work-in-cicada)
- [Windows support?](https://github.com/mitnk/cicada/blob/master/docs/faq.md#user-content-windows-support)
