# cicada Release Notes

## 1.2.2 - 2026.01.23

- Updated builtin `ulimit`.

## 1.2.1 - 2025.12.29

- Fixed path completion issue introduced in previous commits.
- Added `cicada --version` option.

## 1.2.0 - 2025.12.29

- Added new builtin: check
- Improved highlighting and completion after command wrappers like sudo.

## 1.1.4 - 2025.12.18

- Fixed the handling of input that contain Unicode combinations.
  * e.g. command: echo "[woman firefighter](https://emojipedia.org/woman-firefighter)".

## 1.1.3 - 2025.10.01

- Fixed highlight render issue in lightweight terminal apps like "iSH".

## 1.1.2 - 2025.04.19

- Fixed issues in `reverse-i-search`.

## 1.1.1 - 2025.04.10

- Had to fork linefeed to publish to crates.io

## 1.1.0 - 2025.04.10

- Added highlighting for command parts while typing in the terminal.
- Updated `history` builtin item printing from desc to asc.
- When opened in VS Code, make cicada a login shell to load rc-file.

## 1.0.3 - 2024.10.26

- Enabled lto for Rust for a smaller binary size.
- Made commands like "vim $(fzf)" work.
- Fixed an issue of `set -e`
- Fixed an issue of exit code for `cicada foo.sh`.

## 1.0.2 - 2024.09.29

- Revert 1.0.1 to support `armv7-unknown-linux-gnueabihf`.

## 1.0.1 - 2024.09.29

- update version of cicada in the lock file.
- Drop buildability on my RPi3 (revert this commit to build).

## 1.0.0 - 2024.09.29

- Bumped version to 1.0!

## 0.9.40 - 2024.09.26

- Fix a Unicode issue for commands end with `&`.

## 0.9.39 - 2024.09.22

- upgraded deps to support latest Rust.

## 0.9.38 - 2023.08.12

- Supports more colors in the prompt.

## 0.9.37 - 2023.08.12

- upgraded dependency libs.

## 0.9.36 - 2023.02.11

- Fixed issues run `!sort` inside Vim.

## 0.9.35 - 2023.01.15

- Git branch name length in PROMPT can be adjusted.
- Fixed an issue of env names start with `_`
- Fixed an issue of parenthesis in command:
  - e.g. `(echo "foo")`
  - e.g. `foo="(135)"`
- New builtin: unpath

## 0.9.34 - 2022.12.05

- Fixed a job control issue.

## 0.9.33 - 2022.11.18

- added multiple-line mode.
- some completion improvements.

## 0.9.32 - 2022.07.17

- Replaced dep chrono with time.
    - due to https://github.com/rustsec/advisory-db/pull/1082

## 0.9.31 - 2022.07.17

- fixed an escape char `\` handling issue.
    - e.g. `alias c='printf "\ec"'` got `\` char removed.

## 0.9.30 - 2022-06-12

- added builtin: `vox create`
- upgraded some dep-libs for security.

## 0.9.2{8,9} - 2022-02-24

- No changes to previous, just add Cargo.lock into git.

## 0.9.27 - 2022-02-24

- Fixed a minor prompt render issue.

## 0.9.26 - 2022-02-21

- No changes, only a version fix for Termux.

## 0.9.25 - 2022-02-20

- fixed an issue that PATH searching will break when items in it cannot be
  read due to permissions.

## 0.9.24 - 2022-02-14

- updated `PATH` initialization.

## 0.9.23 - 2022-01-22

- updated to use rust edition 2021.
- disable self-defined signal handler by default for safety.
- upgraded dep-libs to latest versions.

## 0.9.22 - 2021-07-24

- Fixed an issue that captured commands does not got reaped.
- Made cicada more stable from crashing.

## 0.9.21 - 2021-07-17

- Fixed issues on exhausting opened files `ulimit -n`.

## 0.9.20 - 2021-07-17

- Fixed a fd leaking issue.
- Established signal handler for SIGCHLD.
- Fixed some issues in job control.

## 0.9.19 - 2021-05-23

- Refine redirections of aliases.
- Fix & imporve redirections/pipelines for builtins.
- Added new beta builtins: set, minfd.

## 0.9.18 - 2021-05-04

- fix compiling issue on 32bit systems.
- Make `history -d` shows local date time.
- Can add optional `; then`, `; do` in heads of `if`, `for`, `while` in scripting.
- Added a new builtin: `read`.
- Added [here string](https://tldp.org/LDP/abs/html/x17837.html).

## 0.9.17 - 2021-01-10

- Fix & improve the builtin `ulimit`.

## 0.9.16 - 2021-01-10

- The shell now ignores signal SIGQUIT and SIGTSTP.
- Added support of `fg %1`, `bg %1` syntax.
- Added builtin `ulimit`.

## 0.9.15 - 2020-11-22

- Fixed pipeline stuck when right hand commands finish first.
- Fixed an cd/pwd issue.

## 0.9.14 - 2020-11-11

- Fixed an env extension bug introduced in 0.9.13
- Made `$gitbr` prompt item searching parent dirs too.

## 0.9.13 - 2020-11-07

- cd: Update `$PWD` when changing directory.
- Added `history add` sub-command.
- Fixed an ENV expension issue.

## 0.9.12 - 2020-05-30

- show full datetime in output of `history -d`.
- Fixed divide by zero panic in arithmetic (e.g. `2 / 0`).
- Arithmetic commands change `previous status` too.

## 0.9.11 - 2020-04-26

- Upgraded some deps.
- Made `HISTORY_DELETE_DUPS=1` as default.

## 0.9.10 - 2020-01-27

- Updated `rc-file` default path in `cinfo`.
- Fixed glob expansion issue: `ls *.1`
- Added more features for builtin `history`.

## 0.9.9 - 2019-10-04

- Improved error messages for running scripts.
- Fixed issue that `ls ~` does not work.
- Fixed filename expansion issue for `2*`.
- Updated math arithmetic recognize rule.
- In scripting, test head's status should not be catched.
- Fixed a completion issue like `echo $USER /App<TAB>`.
- Upgraded dependency libs.

## 0.9.8 - 2019-06-20

- Fixed a double expansion issue: `${1,2}-${foo}`.
- `source` can take extra args now.
- Recognized new RC file location: `~/.config/cicada/cicadarc`.
- Replaced `~/.cicada/` with `~/.config/cicada/`.
- Fixed issue of not closing pipes when running commands.

## 0.9.7 - 2019-05-26

- Added functions ability into scripting.
- Fixed alias expansion when using `xargs`: `foo | xargs ls`.
- Other minor fixes.

## 0.9.6

- Added `if`, `for`, `while` expression into cicada scripting ability.
- Added new braces range expansion: `{1..10}`.
- Fixed a parsing issue for: `alias foo-bar='echo foo bar'`.
- Fixed cannot define single-char-long env/variable.

## 0.9.5

- Added `-l` as an equivalent to `--login`.
- Replaced dep nom 3.0 with pest.
- Replaced dep time with chrono.
- Fixed redirection issue with `echo foo\>bar`.
- Fixed completion issue with `ls \[<TAB>`.
- Fixed issues that on Linux some commands sometimes would `STOPPED` just after start.
- Support math calculation in sub commands: `echo "hi $(1 + 1)"`.

## 0.9.4

- Added basic scripting ability.
- Builtin `source` fully implemented.
- Removed `include` from rcfile, please use `source` instead.
- Added new builtin `alias`, `unalias`.
- Only login shell loads rcfile.
- Some other bug fixes.

## 0.9.3

- Made brace expansion behavior align with bash.
- Two more issues fixes on path completion.
- Partly implemented builtin command `source` (RC loading only).

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
