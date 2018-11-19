## FAQs

### Why another shell?

- for fun
- to learn Rust
- have a shell that can customize a bit for my own needs

### Compare to bash?

Bash is where most people come from and familiar with. So cicada is trying
to support common cases that bash supports. Cicada will only be a "subset"
of bash. Currently cicada does not have scriping/function ability.

### When will functions get supported in cicada?

Maybe someday in future, and it won't be complex as bash scriping for sure.

### Is cicada POSIX-compatible?

As the above answers hints, while cicada is trying to be POSIX, but it will
not be a fully POSIX shell. However, If any command pattern is common and
cicada is missing, we can add it.

### Will my bash/zsh scripts continue work in cicada?

You can invoke scripts with `$ ./my-script.sh` as long as it have
"#!/bin/bash" stuff on the top. Or you can always run them as:
`$ bash my-script.sh`.

Cicada does not recognize these scripts itself. You cannot integrate these
shell scripts/functions in RC files to initiate cicada shell.

### Windows support?

Cicada is a Unix shell, sorry.
