# OCSW 2025 Battleship Example

This is an example of how to use zkVMs, and in particular RISC Zero, used for the Open Source Cryptography Workshop 2025 session on "A Practical Introduction to zkVMs"

## Quick Start

First, make sure [rustup] is installed.
The [`rust-toolchain.toml`][rust-toolchain] file will be used by `cargo` to automatically install the correct version.

This example implements a [battleship game][battleship-wiki] played on the CLI. You can run it with:

```bash
cargo run
```

## Testing

This example includes units tests in the core library, and guest integration tests in the guests crate.
You can run both sets of tests with:

```
RISC0_DEV_MODE=1 cargo test
```

This command runs the tests in [development mode](#development-mode).
Removing this environment variable will run the full proving operations for each test.

### Development mode

RISC Zero has a development mode which turns off both proof generation and **proof verification**.
This is included to be used for testing and iterating on a guest design.

Enabling dev mode happens through by setting the `RISC0_DEV_MODE` environment variable to `1` or `true`.

When deploying to production, the `disable-dev-mode` feature can be enabled on the `risc0-zkvm` crate in any `Cargo.toml`.

## Directory structure

Below is an overview of the project structure and the files in it.
This is a typical pattern for a project, with three crate:

* The `guests` directory contains the code to be compiled into a RISC-V binary for the `riscv32im-zkvm-unkown-elf` target and run in the zkVM.
* The `core` directory contains shared code, which is a library for use both in the guest and in the host code.
* The `host` directory contains the code that turns into a binary run on the host. In this example, it is a CLI.

```
.
├── Cargo.toml
├── host
│   ├── Cargo.toml
│   └── src
│       └── main.rs                   <-- [Host code goes here]
├── core
│   ├── Cargo.toml
│   └── src
│       └── lib.rs                    <-- [Shared code goes here]
└── guests
    ├── Cargo.toml
    ├── build.rs
    ├── battleship
    │   ├── Cargo.toml
    │   ├── src                       <-- [Guest code goes here]
    │   │   ├── init.rs
    │   │   └── round.rs
    │   └── tests
    │       └── example_game.rs
    └── src
        └── lib.rs
```

[rustup]: https://rustup.rs
[rust-toolchain]: rust-toolchain.toml
[battleship-wiki]: https://en.wikipedia.org/wiki/Battleship_(game)
