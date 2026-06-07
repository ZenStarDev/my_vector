# my_vector

`#![no_std]` `Vec` built on `alloc`. Works where `std::Vec` doesn't.

## Install

```toml
[dependencies]
my_vector = "0.1"
```

## Use

```rust
#![no_std]
extern crate alloc;
use my_vector::Vec;

let mut v = Vec::new();
v.push(1);
v.push(2);
assert_eq!(v.pop(), Some(2));
```

## Notes

- 2x growth, panics on OOM
- `remove` is O(n)
- No `shrink_to_fit` yet

MIT / Apache-2.0
