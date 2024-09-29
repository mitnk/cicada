# Install Cicada

There are a few ways to install cicada into your system.

## Install on Some Known Linux Distributions

### Alpine Linux

[https://pkgs.alpinelinux.org/package/edge/community/x86_64/cicada](https://pkgs.alpinelinux.org/package/edge/community/x86_64/cicada)

```
$ sudo apk add cicada -X https://dl-cdn.alpinelinux.org/alpine/edge/community/
```

### Arch Linux

[cicada shell on AUR](https://aur.archlinux.org/packages/cicada-shell)

> Note: `makepkg` will install [rust](https://www.rust-lang.org/) system and
> other packages.

```
$ git clone https://aur.archlinux.org/cicada-shell.git
$ cd cicada-shell/
$ makepkg -si
```

### Termux

Termux is an Android [terminal emulater](https://wiki.termux.com/wiki/Main_Page).

To install cicada in Termux, just input:
```
$ pkg install cicada
```

## Generic Install Options

### Option A: cargo

If [rust](https://rustup.rs/) installed on your system, you can
install cicada with:
```
$ cargo install -f cicada
```

Optionally, you can move the binary downloaded to a common place:
```
$ cp ~/.cargo/bin/cicada /usr/local/bin/cicada
```

### Option B: via Pre-built Binaries

First download the latest right binary for your system from
[Release Page](https://github.com/mitnk/cicada/releases).

Move it to right place and add runable permisson:

```
# on Mac
$ mv cicada-aarch64-apple-darwin /usr/local/bin/cicada

# on Linux
$ mv cicada-x86_64-unknown-linux-gnu /usr/local/bin/cicada

$ chmod +x /usr/local/bin/cicada
```

If you encounter error when running the binary downloaded:

```
... libc.so.6: version `GLIBC_2.31' not found (required by ./cicada)
```

That indicate the GLIBC version on your system is too old.  Please try install
cicada with other options.


### Option C: via Source

Note: [Rust environment](https://rustup.rs/) (Rust stable or above) is required.

```
$ git clone https://github.com/mitnk/cicada
$ cd cicada
# try cicada without installing
$ make
$ make install
```

cicada will be installed under `/usr/local/bin`

> I found on newer MacOS, a reboot is needed after generating a new binary.
> This may be an bug/feature of the OS security things.
>
> UPDATE: a reboot can be avoided if we run:
> `rm -f /usr/local/bin/cicada && cp /path/of/new/cicada /usr/local/bin/`

## Set cicada as your login shell

**WARNING**: Please test cicada on your system before setting it as default
shell.

In file `/etc/shells`, append the following line:

```
/usr/local/bin/cicada
```

Then run

```
$ chsh -s /usr/local/bin/cicada
```

Next time you open a terminal window, cicada shell will be the default one.

### default shell on GNOME Terminal

For GNOME Terminal on Ubuntu or other systems, you may need reboot after
running `chsh`. If failed, another way to config terminal to use cicada is
following:

Preferences --> Profiles (Unnamed) --> Tab Command:
- [checked] Run command as a login shell
- [checked] Run a custom command instead of my shell:
    - Custom command: `cicada -l`

## inputrc

For better terminal use experience, I have following in my `~/.inputrc` file:

```
"\e[1;3D": backward-word
"\e[1;3C": forward-word
"\e[1;9D": backward-word
"\e[1;9C": forward-word
"\e[A": history-search-backward
"\e[B": history-search-forward
```
