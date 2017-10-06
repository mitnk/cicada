# Change Logs

## v0.7.1 - 2017-10-06

- Made cicada also a library.
- More info added in `cinfo` command.

## v0.6.5 - 2017-09-18

- fixed an issue when current dir become not available (e.g. be deleted).
- fixed an issue that commands like `echo "|"` cannot be run.
- Let command `echo 'a * b'` does not extend `*`.
- Added dollar cmd replacement (i.e. `ls -lh $(which bash)`) support.
- Aliases now support cmds like `alias test="echo hi && echo yoo"`

## v0.6.3 - 2017-07-22

- rename builtins command `version` to `cinfo`.
- prehandle command lines from args too.
- fixed a bug of alias expension.

## v0.6.2 - 2017-07-01

- Fixed an issue that `echo a || echo b` was broken.
- Added Env `CICADA_LOG_FILE`.

## v0.6.1 - 2017-07-01

- Pipelines can be used without spaces: `ls|wc`. it was required to run as
  `ls | wc` previously.
- Added support `echo $?` and `echo $$`.

## v0.6.0 - 2017-06-30

- Improved cicada's stability.
- Fixed an issue that `echo ''` would crash.


-------------------------------------------


## v0.5.7 - 2017-06-27

- `export` now can set multiple envs at once
- `echo $NON_EXIST` prints empty string now

## v0.5.6 - 2017-06-25

- Now we can parse following command lines correctly:
    - ``` export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include ```
    - ``` echo "`date` and `go version`" ```
    - ``` echo `date` and `go version` ```
- updated logger

## v0.5.5 - 2017-06-22

- Added `make` completion.
- Added `ssh` completion.
- Fixed an alias extension issue.
