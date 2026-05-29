# actrpc-core-macros

Procedural macros for `actrpc-core`.

## Macros

* `DescribeValue`
* `DescribeParams`
* `DescribeOk`

Example:

```rust
use actrpc_core::{DescribeParams, DescribeOk, DescribeValue};

#[derive(DescribeParams)]
struct Params {
    name: String,
    count: Option<i32>,
}

#[derive(DescribeOk)]
struct ResultData {
    id: String,
}

#[derive(DescribeValue)]
struct Payload {
    enabled: bool,
    tags: Vec<String>,
}
```
