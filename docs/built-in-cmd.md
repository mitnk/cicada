# Cicada Built-in Commands

## cd

Change your current work directory.

## cinfo

Print information of cicada and OS.

## exec

If command is specified, it replaces the shell. No new process is created.
The arguments become the arguments to command.

## export

Change environment variables for current session. You can also use `export` in
`~/.cicadarc` file.

Examples:
```
$ export PATH="/usr/local/bin:$PATH:$HOME/.cargo/bin"
$ export RUST_BACKTRACE=full
$ export PYTHONPATH=.
```

## history

List your recent history:
```
$ history
0: touch docs/envs.md
1: mvim docs/envs.md
2: find . -name '*.bk' | xargs rm
3: find . -name '*.bk'
```
Search history items:
```
$ history curl
0: curl -x http://127.0.0.1:1080 https://hugo.wang/http/ip/
1: curl -x socks5://192.168.1.170:51080 https://hugo.wang/http/ip/
```

## vox

First create your virtual envs under this directory:
```
export VIRTUALENV_HOME="${XDATA_DIR}"
```

Then use `vox` to enter them:
```
$ vox enter my-project
(my-project) $  # now you activated your env
```

List your envs under `$VIRTUALENV_HOME` dir:
```
$ vox ls
```

Exit (deactivate) your env:
```
(my-project) $ vox exit
$  # now you're clean
```
