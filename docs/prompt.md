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
| ${GITBR} | show git branch name (if inside a repo). |
| ${GREEN} | change terminal color to green. |
| ${GREEN_B} | change terminal color to bold green. |
| ${GREEN_BG} | change terminal color to background green. |
| ${RED} | change terminal color to red. |
| ${RED_B} | change terminal color to bold red. |
| ${RED_BG} | change terminal color to background red. |
| ${WHITE} | change terminal color to white. |
| ${WHITE_B} | change terminal color to bold white. |
| ${WHITE_BG} | change terminal color to background white. |
| ${COLOR_STATUS} | change terminal color to `green_b`/`red_b` based on last exit status code. |
| ${BOLD} | make text bold. |
| ${UNDERLINED} | Underlined text. |
| ${RESET} | reset terminal color & format. |

Note you can also use regular environment variables that not in the list, like `$HOME`, in the `$PROMPT` value.

## Python Virtual Env in Prompt

See also the [vox](https://github.com/mitnk/cicada/blob/master/docs/builtins.md#vox) builtin.

When you enter a virtual env, the prompt will prefix by `(pyenv-name)`. e.g.
`(evn-test)mitnk:mbp: pip$ `.

## Use Command Output in Prompt

You can use `$(the cmd line)` in prompt, and the output of command
`the cmd line` will be rendered in prompt. e.g.
```
export PROMPT="[$(git rev-parse --abbrev-ref HEAD)] $USER@HOSTNAME$ "
```
would render prompt with:
```
$ [master] mitnk@mpb$
```

### Use prefix & suffix in it (BETA)
You can use `[` or `{` as prefix, and `]`, `}` as suffix when using command
output int prompt. e.g.
```
export PROMPT="$({git rev-parse --abbrev-ref HEAD}) $USER@HOSTNAME$ "
```
So that when the output of the command is empty, the prefix/suffix wouldn't
be rendered either.

