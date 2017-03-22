# Rush

A simplified shell written by Rust.


## Install (needs rust environment)

```
$ make install
```


## Set rush as your login shell

Appending `/usr/local/bin/rush` into your `/etc/shells`, then run
```
$ chsh -s /usr/local/bin/rush
```


## Features so far

- run programs
- pipelines (barely)
- history (barely)
- math arithmetic (e.g. `1 + 2 * 3 - 4`)


## To do list

- redirections
- and less...


## Won't do list

- job controls (`Ctrl-Z`, `fg`, `bg` etc)
- Windows support
- and more...


## Related projects

- [xonsh](https://github.com/xonsh/xonsh) - A python-powered, cross-platform,
Unix-gazing shell.
