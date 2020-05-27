# Cicada Shell Expansions

## Brace Expansion

```sh
$ echo sp{el,il,al}l
spell spill spall

$ cp foo.txt{,.bak}
# equal to `cp foo.txt foo.txt.bak`

$ echo {1..5}
1 2 3 4 5

$ echo {1..5..2}
1 3 5
```

## Tilde Expansion

```
$ echo ~/foo
# equal to echo $HOME/foo
```

## Parameter Expansion

Currently only works in scripting.

```sh
$ cat foo.sh
echo "the args are: $@"
echo $3 $1 $2
echo $0

$ cicada foo.sh a b c
the args are: a b c
c a b
foo.sh
```

## Command Substitution

Command substitution allows the output of a command to replace the command
itself. Command substitution occurs when a command is enclosed as follows:

```
$(command)
```

or
```
`command`
```

## Filename Expansion

```
$ echo src/*.rs
src/build.rs src/execute.rs src/history.rs src/jobc.rs ...
```

## Special Expansions

```sh
# current session process ID
$ echo $$
26653

# last command exit status
$ echo $?
0

$ cat /etc/some-config

# last command substitution
$ sudo !!
sudo cat /etc/some-config
```
