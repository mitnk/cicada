# Introduction

Cicada is a simple bash-like Unix shell.

### Try out cicada with Docker

```
$ docker pull mitnk/cicada
$ docker run --rm -it mitnk/cicada
(in-cicada) $ cinfo
```

## Basic cicada Features

### Run programs and pipelines

```
$ ls | head -n3
Desktop
Documents
Downloads

$ echo foo,bar | awk -F "," '{print $2, $1}'
bar foo
```

### With redirections

```
$ ls file-not-exist 2>&1 | wc > e.txt
$ cat e.txt
       1       7      46

$ cat <<< 'here string'
here string
```

### With heredocs

Feed a command lines typed right after it, up to a delimiter of your choice.
Cicada prompts with `>> ` until the delimiter shows up on a line of its own:

```
$ cat << EOF > foo.txt
>> hello
>> world
>> EOF

$ cat >> foo.txt << END
>> one more line
>> END
```

The delimiter line must be exactly the delimiter, with nothing else on it --
a trailing space keeps the heredoc open. Bodies take the usual expansions:

```
$ cat << EOF
>> today is $(date +%A), $USER
>> EOF
today is Sunday, mitnk
```

Quote the delimiter to turn expansion off and pass the body through as typed.
Quoting any part of it will do -- `<<'EOF'`, `<<"EOF"` and `<<\EOF` all wait
for a line reading `EOF`:

```
$ cat <<'EOF' | sed 's/l/e/g'
>> Hello
>> World
>> EOF
Heeeo
Wored
```

With `<<-` instead of `<<`, leading tabs are stripped from the body and from
the delimiter line, so a heredoc inside an indented block can be indented too
(with tabs -- spaces are body text):

```
$ cat <<- EOF
>> 	alpha
>> 	EOF
alpha
```

### Command substitution

```
$ ls -l `which sh`
-r-xr-xr-x  1 root  wheel  618512 Oct 26  2017 /bin/sh

$ echo "Time is $(date)."
Time is Sun Sep  2 12:04:13 CST 2018.
```

### Run multiple commands (with logical)

```
$ echo foo; echo bar
foo
bar

$ echo foo && echo bar
foo
bar

$ echo foo || echo bar
foo
```

### Shell expansions

```
$ echo sp{el,il,al}l
spell spill spall

$ echo $SHELL
/usr/local/bin/cicada

$ echo *
Cargo.lock Cargo.toml LICENSE Makefile README.md src target
```

### Math arithmetic directly in the shell!

```
$ 1 + 2 * 3 - 4
3
$ (1 + 2) * (3 - 4) / 8.0
-0.375
$ 2 ^ 31
2147483648
```
