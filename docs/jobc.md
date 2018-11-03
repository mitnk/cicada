# Job Control

In a single cicada session, you can run commands in background, and bring them
foreground when needed.

For example when download a file with `wget`:

```
$ wget 'https://speed.hetzner.de/100MB.bin'
```

We found it is too slow, we want it run in background instead. With
cicada (just like bash), you could achieve it like this:

```
# press `Ctrl-Z` to stop it
$ wget 'https://speed.hetzner.de/100MB.bin'
^Z
[1] 38273  Stopped    wget 'https://speed.hetzner.de/100MB.bin'
```

Then let's continue it running in background with builtin command `bg`:

```
$ bg
wget 'https://speed.hetzner.de/100MB.bin' &
```

You can check the job status with command `jobs`:

```
$ jobs
[1] 38273  Running    wget 'https://speed.hetzner.de/100MB.bin' &
```

Now you can start another job while `wget` is downloading. Let's download a
even bigger file in background directly:

```
$ wget 'https://speed.hetzner.de/1GB.bin' &
[2] 38337

$ jobs
[2] 38337  Running    wget 'https://speed.hetzner.de/1GB.bin' &
[1] 38273  Running    wget 'https://speed.hetzner.de/100MB.bin' &
```

If you want to stop the `100M` file downloading. You can bring it foreground
and then use `Ctrl-C` to terminate it.

```
$ fg 1
wget 'https://speed.hetzner.de/100MB.bin'
^C

$ jobs
[2] 38337  Running    wget 'https://speed.hetzner.de/1GB.bin' &
```

The number `1` in `fg 1`, is the job id, which shows in `jobs` command,
indicating which job we want to bring.

The number `38273` is the process group id of the job. `fg 28273` is an
alternative to `fg 1` here.
