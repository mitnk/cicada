# Use cicada as a Library

See latest API Docs here: [https://docs.rs/cicada/](https://docs.rs/cicada/)

## Add cicada into your `Cargo.toml`

```
[dependencies]
cicada = "0.8.0"
```

## Use cicada functions

```rust
// file content of src/main.rs:
extern crate cicada;

fn main() {
    let tokens = cicada::cmd_to_tokens("echo 'hi yoo' | `which wc`");
    assert_eq!(tokens.len(), 4);

    assert_eq!(tokens[0].0, "");
    assert_eq!(tokens[0].1, "echo");

    assert_eq!(tokens[1].0, "'");
    assert_eq!(tokens[1].1, "hi yoo");

    assert_eq!(tokens[2].0, "");
    assert_eq!(tokens[2].1, "|");

    assert_eq!(tokens[3].0, "`");
    assert_eq!(tokens[3].1, "which wc");

    let out1 = cicada::run("ls Cargo.toml foo");
    assert_eq!(out1.status, 1);
    assert_eq!(out1.stdout, "Cargo.toml\n");
    assert_eq!(out1.stderr, "ls: foo: No such file or directory\n");

    let out2 = cicada::run("ls | wc");
    assert_eq!(out2.status, 0);
    assert_eq!(out2.stdout, "       4       4      33\n");
}
```
