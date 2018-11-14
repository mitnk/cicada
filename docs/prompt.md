# Customize Prompt in Cicada Shell

In bash, you can customize your shell prompt by setting `PS1` environment
variable. In cicada, we can do this by setting `PROMPT` env in `~/.cicadarc`.

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
Note `$XYZ` is the same as `${XYZ}`, but sometime you need the `${XYZ}` form
to achieve prompt string like `FOO${XYZ}BAR`, where it would be treated as
`$XYZBAR` without the `{}` chars. Also, `$XYZ` is the same as `$xyz`.

| Prompt Item | Description |
| --- | --- |
| ${BLUE} | change terminal color to blue. |
| ${COLOR_STATUS} | change terminal color to green/red based on last exit status code. |
| ${CWD} | current work directory base name. e.g. `baz` for dir `/foo/bar/baz`. |
| ${GREEN} | change terminal color to green. |
| ${HOSTNAME} | system hostname. |
| ${NEWLINE} | the newline char: `\n`. |
| ${RED} | change terminal color to red. |
| ${RESET} | reset terminal color. |
| ${USER} | system user name. |

Note you can also use regular environment variables that not in the list, like `$HOME`, in the `$PROMPT` value.

## Python Virtual Env in Prompt

See also [builtin vox](https://github.com/mitnk/cicada/blob/master/docs/built-in-cmd.md#vox)

When you enter a virtual env, the prompt will prefix by `(pyenv-name)`. e.g.
`(evn-test)mitnk:mbp: pip$ `.
