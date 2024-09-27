# insertion-set

[![Crates.io Version](https://img.shields.io/crates/v/insertion-set?style=for-the-badge)](https://crates.io/crates/insertion-set)
[![docs.rs](https://img.shields.io/docsrs/insertion-set?style=for-the-badge)](https://docs.rs/insertion-set)

<!-- cargo-rdme start -->

Performs a set of batched insertions on a vector.

[`Vec::insert(index, value)`][Vec::insert] takes `O(n)` time to move internal memory,
so calling it in a loop can cause quadratic blowup.

If you batch multiple values together with an [`InsertionSet`]
you can defer the expensive movement of the vector's
memory till the of the loop.

This code was originally copied from the first prototype compiler for [DuckLogic].
It was inspired by the way the [B3 JIT] handles insertions.

[DuckLogic]: https://ducklogic.org/
[B3 JIT]: https://webkit.org/blog/5852/introducing-the-b3-jit-compiler/

<!-- cargo-rdme end -->

<!-- TODO: Make `cargo-rdme` good enough to infer these -->
[`InsertionSet`]: https://docs.rs/insertion-set/latest/insertion-set/struct.InsertionSet.html
[Vec::insert]: https://doc.rust-lang.org/std/vec/struct.Vec.html#method.insert

## License
Licensed under either of Apache License, Version 2.0 or MIT license at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Serde by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
