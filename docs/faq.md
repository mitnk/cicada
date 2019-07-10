## FAQs

### Why another shell?

- <del>for fun</del>
- <del>to learn Rust</del>
- <del>have a shell that I can customize a bit for my own needs</del>

Because we can. ☺

### Compare to Other Shells

Bash is where most people come from and what they are familiar with.
Cicada is trying to support most common cases that Bash supports, except
the Bash scripting language.

In following aspects, cicada wants to do a better (sanity) job:

- [Completion](https://github.com/mitnk/cicada/tree/master/docs/completion.md)
- [History](https://github.com/mitnk/cicada/tree/master/docs/history.md)
- [Customize Prompt](https://github.com/mitnk/cicada/tree/master/docs/prompt.md)

Compare to shells like zsh/fish, cicada tends to be a simpler shell.
Audience of cicada shell should be people that are seeking simplicity and
speed, while not full-feature.

### Is cicada POSIX-compatible?

As the above answers hints, while cicada is trying to be POSIX, it will
not be a fully POSIX shell. However, if any command pattern is common and
cicada is missing support, we could add it.

### Will my bash/zsh scripts continue work in cicada?

You can invoke scripts with `$ ./my-script.sh` as long as they have a shebang
(`#!/bin/bash`) at the top. Or you can always run them as:
`$ bash my-script.sh`.

Cicada does not recognize these scripts itself. You cannot integrate these
shell scripts/functions in RC files to initiate cicada shell. But you could
use [the scripting ability of cicada](https://github.com/mitnk/cicada/tree/master/docs/scripting.md).

### Windows support?

Cicada is a Unix shell, sorry. There are a lot of alternative cool shells
for Windows, for example [xonsh](https://xon.sh/).
