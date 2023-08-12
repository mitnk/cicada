# Customize Prompt

In bash, you can customize your shell prompt by setting `PS1` environment
variable. In cicada, we can do this by setting `PROMPT` env in
[RC-file](https://github.com/mitnk/cicada/blob/master/docs/rc-file.md).

By default, `$PROMPT` uses the following value:
```
export PROMPT="${COLOR_STATUS}$USER${RESET}@${COLOR_STATUS}$HOSTNAME${RESET}: ${COLOR_STATUS}$CWD${RESET}$ "
```
The prompt will look like this:
```
username@hostname: current-dir$
```

## Available Prompt items

A prompt item is prompt value fields like `$USER`, `${COLOR_STATUS}` etc.
Note `$XYZ` is the same as `${XYZ}`, and `$XYZ` is the same as `$xyz`.

| Prompt Item | Description |
| --- | --- |
| ${CWD} | current work directory base name. e.g. `baz` for dir `/foo/bar/baz`. |
| ${HOSTNAME} | system hostname. |
| ${NEWLINE} | the newline char: `\n`. |
| ${USER} | system user name. |
| ${BLACK} | change terminal color to black. |
| ${BLACK_B} | change terminal color to bold black. |
| ${BLACK_BG} | change terminal color to background black. |
| ${BLUE} | change terminal color to blue. |
| ${BLUE_B} | change terminal color to bold blue. |
| ${BLUE_BG} | change terminal color to background blue. |
| ${BLUE_L} | change terminal color to background light blue. |
| ${BLUE_L_BG} | change terminal color to background light blue. |
| ${GITBR} | show git branch name (if inside a repo). |
| ${GRAY} | change terminal color to gray. |
| ${GRAY_D} | change terminal color to dark gray. |
| ${GREEN} | change terminal color to green. |
| ${GREEN_B} | change terminal color to bold green. |
| ${GREEN_BG} | change terminal color to background green. |
| ${GREEN_L} | change terminal color to background light green. |
| ${GREEN_L_BG} | change terminal color to background light green. |
| ${RED} | change terminal color to red. |
| ${RED_B} | change terminal color to bold red. |
| ${RED_BG} | change terminal color to background red. |
| ${RED_L} | change terminal color to background light red. |
| ${RED_L_BG} | change terminal color to background light red. |
| ${WHITE} | change terminal color to white. |
| ${WHITE_B} | change terminal color to bold white. |
| ${WHITE_BG} | change terminal color to background white. |
| Other Colors | Others color names available: `$CYAN`, `$MAGENTA`, `$GRAY`, `$GRAY_D`, etc. Most of them can add suffixes: `_L`, `_BG`, `_L_BG` etc |
| ${COLOR_STATUS} | change terminal color to `green_b`/`red_b` based on last exit status code. |
| ${BOLD} | make text bold/bright. |
| ${DIM} | Make text dim. |
| ${HIDDEN} | Make text hidden. |
| ${BLINK} | Make text blink. |
| ${UNDERLINED} | Underlined text. |
| ${REVERSE} | Invert the foreground and background colors. |
| ${RESET} | reset terminal color & format. |
| ${RESET_BLINK} | reset blink. |
| ${RESET_BOLD} | reset bold. |
| ${RESET_DIM} | reset dim. |
| ${RESET_HIDDEN} | reset hidden. |
| ${RESET_REVERSE} | reset reverse. |
| ${RESET_UNDERLINED} | reset underlined. |
| ${SEQ} | Starts a terminal escape sequence |
| ${END_SEQ} | Ends a terminal escape sequence |
| ${ESC} | Represents a `\e` char in an escape sequence. |

Note you can also use regular environment variables that not in the list, like `$HOME`, in the `$PROMPT` value.

## Use Extra Colors

If you try following command in your terminal:
```sh
$ bash -c 'echo -e "\e[1;31;42m Hello \e[30;48;5;82m World \e[0m"'
```

You would see effects like this:
![256 colors](https://raw.githubusercontent.com/mitnk/cicada/master/misc/prompt-256-hello.png)
Check [here](https://misc.flogisoft.com/bash/tip_colors_and_formatting)
for more colors.

You could do the same theme by defining `PROMPT` in your
[RC-file](https://github.com/mitnk/cicada/tree/master/docs/rc-file.md) like
following:
```sh
export PROMPT="${SEQ}${ESC}[1;31;42m${END_SEQ} $USER ${SEQ}${ESC}[30;48;5;82m${END_SEQ} $CWD ${RESET} "
```

You need to put those colors sequences into pair of `$SEQ / $END_SEQ`, and also
need to use `$ESC` as the char of Escape char `\e`.

## Python Virtual Env in Prompt

See also the [vox](https://github.com/mitnk/cicada/blob/master/docs/builtins.md#user-content-vox) builtin.

When you enter a virtual env, the prompt will prefix by `(pyenv-name)`. e.g.
`(evn-test)mitnk:mbp: pip$ `.

## Use sub-Command Output in Prompt

You can use `$(the cmd line)` in prompt, and the output of command
`the cmd line` will be rendered in prompt. e.g.
```
export PROMPT="[$(git rev-parse --abbrev-ref HEAD)] $USER@HOSTNAME$ "
```
would render prompt with:
```
$ [master] mitnk@mpb$
```

> WARNING: when
> [CICADA_ENABLE_SIG_HANDLER](https://github.com/mitnk/cicada/blob/master/docs/envs.md#cicada_enable_sig_handler)
> is enabled, use in-line command in prompt could cause cicada crash, with
> error message like `BUG IN CLIENT OF LIBPLATFORM: Trying to recursively lock
> an os_unfair_lock`. View [detailed logs](https://pastebin.com/3krRLUNp)
