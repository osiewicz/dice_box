# Dice_box - a testing ground for better cargo scheduler

Dice_box allows one to check expected build time (makespan length) of a given Rust project with different Cargo scheduling algoritms.
That is mostly motivated by a want and need to test out different scheduling algorithms without having to run the builds (which should make iteration on new algorithms quicker).
However, it should also give you an idea of an expected build speedup and scalability with different # of CPU cores.

## Getting started

To run Dice_box on your project, you need 2 files:
- Build timings of your project: `cargo +nightly build --timings=json`
- Unit graph of your project: `cargo +nightly build --unit-graph`
which can then be passed into a Dice_box:
`dice_box timings.json unit_graph.json`

It is also possible to control parallelism of a build with `-n` option; this affects the results of Dice_box, not it's speed of execution (which should generally be quick-enough). It simulates a build of a given crate with N threads, where N defaults to 10.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

at your option.
