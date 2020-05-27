# Arithmetic in Cicada

Arithmetic is directly available in cicada.

```
$ 1 + 1
2

$ 3 * 7
21

$ 7 / 3
2

$ 2 ^ 16 - 1
65535
```

## Using Parentheses

```
$ (1 + 1) * 7
14
```

## Float Calculation

Whenever there is a `.` in command, the whole calculation will be in float.

```
$ 7.0 / 3
2.3333333333333335

$ 7.0 / 3 + 10000
10002.333333333334
```

## Single Numbers

A single number will not be treated as arithmetic. It will be treated as
a regular command.

```
$ 42
cicada: 42: command not found
```
