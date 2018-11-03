# Cicada Built-in Commands

## bg

Make stopped job runing in background. See also `fg`, `jobs`.

## cd

Change your current work directory.

## cinfo

Print information of cicada and OS.

## exec

If command is specified, it replaces the shell. No new process is created.
The arguments become the arguments to command.

## exit

Exit the current progress (the shell). Can exit with an extra code like:
`exit 2`.

## export

Change environment variables for current session. You can also use `export` in
`~/.cicadarc` file.

Examples:
```
$ export PATH="/usr/local/bin:$PATH:$HOME/.cargo/bin"
$ export RUST_BACKTRACE=full
$ export PYTHONPATH=.
```

## fg

Bring background job into foreground. See also `bg`, `jobs`.

## history

List your recent history:
```
$ history
0: touch docs/envs.md
1: mvim docs/envs.md
2: find . -name '*.bk' | xargs rm
3: find . -name '*.bk'
```

Search history items (use `%` from SQL to match "anything"):
```
$ history curl
0: curl -x http://127.0.0.1:1080 https://hugo.wang/http/ip/
1: curl -I https://twitter.com/

$ history 'curl%hugo'
0: curl -x http://127.0.0.1:1080 https://hugo.wang/http/ip/
```

## jobs

Listing all jobs in [job control](https://github.com/mitnk/cicada/blob/master/docs/jobc.md).
See also `bg`, `fg`.

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
