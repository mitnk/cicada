## Install Cicada

### Install Cicada via Source

Note: [Rust environment](https://rustup.rs/) is needed for installation.

You can try `cicada` out without installing it by checking out the repository
and run `cargo run` in its root directory.

```
$ git clone https://github.com/mitnk/cicada
$ cd cicada
$ cargo run
```

### install from code repository

If you've checked out the cicada repository, you can do this:

```
$ make install
```

This will install `cicada` under your `/usr/local/bin`. Use `sudo` if needed.

### install via cargo crates

```
$ cargo install -f cicada
```

This will install cicada into `~/.cargo/bin/`.

### Install via Pre-built Binaries

First download the latest right binary for your system from
[Release Page](https://github.com/mitnk/cicada/releases).

Move it to right place and add runable permisson:

```
# on Mac
$ mv cicada-mac-v0.6.5 /usr/local/bin/cicada

# on Linux
$ mv cicada-linux64-v0.6.5 /usr/local/bin/cicada

$ chmod +x /usr/local/bin/cicada
```

Then you can try it by run `cicada` in your shell.

### Set cicada as your login shell

WARNING: Please test cicada on your system before set it as default shell.

Appending `/usr/local/bin/cicada` into your `/etc/shells`, then run
```
$ chsh -s /usr/local/bin/cicada
```
