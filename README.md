# softfloat-sys

Rust bindings to Berkeley softfloat library written in C

## Requirements

* Rust 1.64 - due to stabilized `core::ffi` types
* `rustfmt` component - `bindgen` uses it to prettify generated bindings
* `clang` compiler and `libclang` - used by `bindgen` to generate C type aliases from C header

## Supported targets:

* Linux x86-64
* Wasm32

Other platforms are supported, though appropriate target branches not added to `build.rs`.
See `berkeley-softfloat-3/build` for list of properly defined targets, except `template-*` subfolders.
Please note that makefiles are not used, so you'll need to copy target-specific settings from respective
`Makefile` to `build.rs`.
