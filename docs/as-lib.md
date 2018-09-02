## Run a command or a pipeline.


```rust
// file content of src/main.rs:
extern crate cicada;

fn main() {
    let out1 = cicada::run("ls").unwrap();
    println!("out1: {:?}", out1.stdout);

    let out2 = cicada::run("ls | wc").unwrap();
    println!("out2: {:?}", out2.stdout);

    let out3 = cicada::run("date >> out.txt").unwrap();
    println!("out3: {:?}", out3.stdout);

    let out4 = cicada::run("cat out.txt").unwrap();
    println!("out4: {:?}", out4.stdout);
}
```

Output:

```
out1: "Cargo.lock\nCargo.toml\nsrc\ntarget\n"
out2: "       4       4      33\n"
out3: ""
out4: "Fri Oct  6 14:53:25 CST 2017\n"
```
