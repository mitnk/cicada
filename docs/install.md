## Install Cicada

There are a few methods to install cicada into your system.

### 1) Install via Pre-built Binaries

First download the latest right binary for your system from
[Release Page](https://github.com/mitnk/cicada/releases).

Move it to right place and add runable permisson:

```
# on Mac
$ mv cicada-mac-0.9.2 /usr/local/bin/cicada

# on Linux
$ mv cicada-linux-0.9.2 /usr/local/bin/cicada

$ chmod +x /usr/local/bin/cicada

# try it
$ cicada
(in-cicada) $ cinfo
```

You may want to [set cicada as the default shell](https://github.com/mitnk/cicada/blob/master/docs/install.md#set-cicada-as-your-login-shell).

### 2) Install via cargo crates

If you already have [Rust environment](https://rustup.rs/), you can install
cicada with `cargo`:

```
$ cargo install -f cicada
```

This will install cicada into `~/.cargo/bin/`.

```
$ mv ~/.cargo/bin/cicada /usr/local/bin/
$ cicada
```

### 3) Install via Source

Note: [Rust environment](https://rustup.rs/) is required.

```
$ git clone https://github.com/mitnk/cicada
$ cd cicada
# try cicada without installing
$ make
# install cicada into /usr/local/bin/cicada
$ make install
$ cicada
```

### Set cicada as your login shell

**WARNING**: Please test cicada on your system before set it as default shell.

In file `/etc/shells`, append the following line:

```
/usr/local/bin/cicada
```

Then run

```
$ chsh -s /usr/local/bin/cicada
```

Next time you open a terminal window, cicada shell will be the default one.
