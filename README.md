# xv6-rust

This is a project to port the popular 32-bit learner's OS, xv6, over to the Rust programming language. This project is forked from the [original work](https://github.com/connorkuehl/xv6-rust) by `connerkuehl`. 

Original documentation for xv6 can be found at: https://pdos.csail.mit.edu/6.828/2012/xv6.html 

The original work is a read-only repo dated back in 2019. Evan Older (CSC525 Fall 2021) provided the corrected version of Cargo dependencies to make the project runs. 

One of the motivating academic factors behind this project (which has played a huge role in making this project possible for school credit) is assessing Rust's viability as a low level systems language.

# Building and Running

Prerequisites:

1. A linux environment.

1. The QEMU simulator.

1. The `gcc` compiler suite.

1. The Rust compiler.

1. A nightly override for the cloned repository (`rustup override set nightly`).

1. The Rust source (`rustup component add rust-src`).

Building:

1. Run `make`.

Running:

1. Run `make run`.
2. To quit, first hit `Ctrl-A`, then `C` to get to qemu's terminal. Type `quit` and hit `Enter` to quit. 

Debugging:

1. Run `make debug`; QEMU will expose a debugging port for GDB to attach to.

2. In another terminal session, run `rust-gdb target/kernel/kernel`.
3. In rust-gdb run `target remote localhost:1234`
