# Change Logs

## 0.9.3 - master

- Made brace expansion behavior align with bash.
- Fixed an issue of path completion in middle of input.
- Implemented builtin command `source`

## 0.9.2

- Wrap prompt when it's too long.
- Replace dep crate `sqlite` with `rusqlite`.
- Fixed a completion issue for paths include unicode.
- Fixed an alias expansion issue.
- Changed to use Rust 2018.
- Correct behavior of `foo; echo $?`.

## 0.9.1

- Skip hidden files when expanding `foo/*`.
- Support `include` in rc file.
- Added completion for ENV.
- Added new prompt item: `$GITBR`.

## v0.9.0

- Works on escape file names.
- Some improvements on command line parser (escape chars etc).
- Added suport for customizing prompt.

## v0.8.9

- Fixed issue of finding command in `$PATH`.
- Fixed issue of cmds like `(ls)`.
- Fixed stuck issue of: `sort < foo.txt`.
- Some improvements on history file init.
- Improved path completion on chars needing escape.
- Make command `touch "foo"/bar.txt` works as expected.

## v0.8.8

- Some enhancement on job control.
- Added builtins `bg`.

## v0.8.7

- Drop use of `std::process::Command`.
- Added job control.
- Added builtins: `fg`, `jobs`.

## v0.8.6

- Fix some minor issues of processes exiting status.

## v0.8.5

- Updated history item display.
- Added history delete.
- Added completion sub-command feature.

## v0.8.4

- Fixed `$$` and `$?` extension.

## v0.8.3

- Fixed path completion issue when contains spaces.
- Fixed environment variable manipulating issues.

## v0.8.2

- Skip from saving into history for commands start with spaces.
- Fixed parser issue on commands like: `mv a\ b xy`.
- Fixed issue that when extending globs when file name contains spaces.
- Fixed issue that rm will fail in `touch foo\ bar.txt && rm foo*`.

## v0.8.1

- Minor updates on `cinfo`.
- Fixed a path completion bug.
- Fixed parsing strings like `foo'bar baz'`.
- Upgraded linefeed to 0.5.

## v0.8.0

- Added support for `!!` (the last command string, eg. `sudo !!`).
- Fixed an issue on glob extending.
- Removed `os_type` from dependency.
- Updated `cicada::run()` API (BETA).
- Added completion on aliases and builtins.
- Better support for stdio redirection.

## v0.7.4

- Improved completion on soft links on directories.
- Upgraded linefeed to `0.4.0`.
- Removed binding of `history-search-forward`.

## v0.7.3

- Fixed a glob bug like `ls ../*.md`.
- Upgraded `linefeed` to its latest master to fix a cmd line length issue.
- Make `Ctrl-D` exit cicada; and added env `NO_EXIT_ON_CTRL_D`.
- Added `exit` as a built-in.
- Extend brace before globbing, for cmds like `echo {a,b,c}*`.
- Fixed a line parsing bug on strong quote `'`.

## v0.7.2 - 2017-10-06

- Renamed lib API `line_to_tokens()` to `cmd_to_tokens()`.
- Added new lib API `is_valid_input()`.

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
