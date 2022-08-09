# Install Cicada

There are a few ways to install cicada into your system.

## Install on Some Known Linux Distributions

### Alpine Linux

[https://pkgs.alpinelinux.org/package/edge/testing/x86_64/cicada](https://pkgs.alpinelinux.org/package/edge/testing/x86_64/cicada)

```
$ apk add cicada -X https://dl-cdn.alpinelinux.org/alpine/edge/testing/
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
$ mv cicada-mac-0.9.2 /usr/local/bin/cicada

# on Linux
$ mv cicada-linux-0.9.2 /usr/local/bin/cicada

$ chmod +x /usr/local/bin/cicada
```

> Due to too many existing CPU arch and different libc versions on same system,
> I tend to stop providing pre-built binaries. Please consider to use other
> options.

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
