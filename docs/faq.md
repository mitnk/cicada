## FAQs

### Why another shell?

- for fun
- to learn Rust
- have a shell that can customize a bit for my own needs

I think it's fair to say cicada is just a toy.

### Compare to bash?

Bash is where most people come from and familiar with. So cicada is trying to support common cases that bash supports. Cicada will only be a "subset" of bash. If bash is a steamship, cicada is just a boat.

### Compare to ion?

[Ion](https://github.com/redox-os/ion) is a modern system shell that is also written in Rust. It's more mature as a general shell. Ion is to Rust what [xonsh](http://xon.sh) to Python, which supports following stuff:
```
$ let string = "one two three"
$ echo $string[0]
o
$ echo $string[..3]
one
```
While cicada do not and will not support these features.

### Why functions support is in won't do list?

I don't think i have interests or energy to add (bash) functions support or (bash) shell scripting ability. If you're a heavy function/scripting user, cicada may not be your tool. If you found cicada useful, you can always add your things based on it.

As far as I can see, cicada will not introduce such complex things, and will not be another zsh/fish.

### Will cicada be POSIX-compatible?

As the above answers hints, while cicada is trying to be POSIX, but it will not be a fully POSIX shell. However, If any command pattern is common and cicada is missing, we can add it.

### Will my bash/zsh scripts continue work in cicada?

It depends. If the script is only doing external things, like an configure/installation script, you can still run it. You can invoke scripts with `$ ./my-script.sh` as long as it have "#!/bin/bash" stuff on the top. Or you can always run them as: `$ bash my-script.sh`.

Cicada does not recognize these scripts itself. You cannot integrate these shell scripts/functions in RC files to initiate cicada shell.

### Windows support?

Cicada is a Unix shell.
