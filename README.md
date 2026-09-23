# httparse

[![crates.io](https://img.shields.io/crates/v/ntex-httparse.svg)](https://crates.io/crates/ntex-httparse)
[![Released API docs](https://docs.rs/ntex-httparse/badge.svg)](https://docs.rs/ntex-httparse)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![CI](https://github.com/fafhrd91/ntex-httparse/workflows/CI/badge.svg)](https://github.com/fafhrd91/ntex-httparse/actions?query=workflow%3ACI)

A push parser for the HTTP 1.x protocol. Avoids allocations. No copy. **Fast.**

Works with `no_std`, simply disable the `std` Cargo feature.

This fork adds runtime AVX-512BW scanning for long URI/header values and AVX-512VBMI
token classification for long header names. CPU and OS feature detection guards
each target-specific entry point. Short fields retain the existing fast path;
AVX2, SSE4.2, AArch64 NEON, and scalar fallbacks remain available. `no_std` uses
the compile-time target features. Rust 1.89 or newer is required.

`make test` checks byte classes, vector boundaries, and guarded-page tails on each
available backend. `make bench` runs the optional scanner microbenchmark.

[Changelog](https://github.com/fafhrd91/ntex-httparse/releases)

## Usage

```rust
let mut headers = [httparse::EMPTY_HEADER; 64];
let mut req = httparse::Request::new(&mut headers);

let buf = b"GET /index.html HTTP/1.1\r\nHost";
assert!(req.parse(buf)?.is_partial());

// a partial request, so we try again once we have more data

let buf = b"GET /index.html HTTP/1.1\r\nHost: example.domain\r\n\r\n";
assert!(req.parse(buf)?.is_complete());
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
