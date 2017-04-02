# mtsh

A simplified shell written by Rust.


## Install (needs rust environment)

```
$ make install
```


## Set mtsh as your login shell

Appending `/usr/local/bin/mtsh` into your `/etc/shells`, then run
```
$ chsh -s /usr/local/bin/mtsh
```


## Features so far

- run programs
- pipeline
- redirections
- history
- math arithmetic (e.g. `1 + 2 * 3 - 4`)


## To do list

- update ENV vars
- completions
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
