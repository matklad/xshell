# xshell: Making Rust a Better Bash

`xshell` provides a set of cross-platform utilities for writing ergonomic "bash" scripts.

```rust
use xshell::{cmd, read_file};

let name = "Julia";
let output = cmd!("echo hello {name}!").read()?;
assert_eq!(output, "hello Julia!");

let err = read_file("feeling-lucky.txt").unwrap_err();
assert_eq!(
    err.to_string(),
    "`feeling-lucky.txt`: no such file or directory (os error 2)",
);
```

See [the docs](https://docs.rs/xshell) for more.
