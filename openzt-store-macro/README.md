# openzt-store-macro

A derive macro that implements the `lrpc::Store` trait while skipping array fields.

## Usage

```rust
use openzt_store_macro::StoreSkipArrays;

#[derive(StoreSkipArrays)]
struct MyStruct {
    id: u32,          // Will be stored/loaded
    data: [u8; 16],   // Will be skipped
    name: String,     // Will be stored/loaded
}
```

## Behavior

- Non-array fields are stored and loaded normally using their `Store` implementation
- Array fields are skipped during `store()` operations
- Array fields are initialized with zeroed memory during `load()` operations

## Why?

The standard `CommonStore` derive macro from `lrpc` doesn't work with arrays. This macro provides a workaround by skipping array fields entirely, which is useful when arrays are used for padding or non-essential data.