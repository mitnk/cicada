## FAQs

### Why another shell?

- <del>for fun</del>
- <del>to learn Rust</del>
- <del>have a shell that can customize a bit for my own needs</del>

Because we can. ☺

### Compare to bash?

Bash is where most people come from and familiar with. So cicada is trying
to support common cases that bash supports. Cicada will only be a "subset"
of bash. <del>Currently cicada does not have scripting/function ability.</del>

### When will functions get supported in cicada?

<del>Maybe someday in future, and it won't be complex as bash scriping for sure.</del>
Scripting/functions **have been added** since cicada 0.9.7.

### Is cicada POSIX-compatible?

As the above answers hints, while cicada is trying to be POSIX, but it will
not be a fully POSIX shell. However, if any command pattern is common and
cicada is missing, we could add it.

### Will my bash/zsh scripts continue work in cicada?

You can invoke scripts with `$ ./my-script.sh` as long as it has the
"#!/bin/bash" shebang. Or you can always run them as:
`$ bash my-script.sh`.

Cicada does not recognize these scripts itself. You cannot integrate these
shell scripts/functions in RC files to initiate cicada shell. But you could
use [the scripting ability of cicada](https://github.com/mitnk/cicada/tree/master/docs/scripting.md).

### Windows support?

Cicada is a Unix shell, sorry. There are a lot of alternative cool shells
for Windows, for example [xonsh](https://xon.sh/).
