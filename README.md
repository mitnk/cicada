# mtsh

A shell written by Rust.


## Install (needs rust environment)

```
$ make install
```


## Set mtsh as your login shell

Appending `/usr/local/bin/mtsh` into your `/etc/shells`, then run
```
$ chsh -s /usr/local/bin/mtsh
```


## Usage (Features so far)

### run programs

```bash
$ ls
Desktop
Documents
Downloads
Dropbox
Games
Library
Movies
Music
...
```

### pipeline

```bash
$ man awk | awk -F "[ ,.\"]+" '{for(i=1;i<=NF;i++)A[$i]++}END{for(k in A)print k, A[k]}' | sort -k2nr | head -n8
the 70
of 40
a 27
is 27
and 24
are 21
in 21
to 21
```

### redirections

```bash
$ ls file-not-exist 2>&1 | wc > e.txt
$ cat e.txt
       1       7      46
```

### math arithmetic

```bash
$ 1 + 2 * 3 - 4
3
$ (1 + 2) * (3 - 4) / 8.0
-0.375
```

### history

see doc

### completions

see doc


## To do list

- update ENV vars
- rc file
- and less...


## Won't do list

- functions
- job controls (`Ctrl-Z`, `fg`, `bg` etc)
- Windows support
- and more...


## Related projects

- [xonsh](https://github.com/xonsh/xonsh) - A python-powered, cross-platform,
Unix-gazing shell.
