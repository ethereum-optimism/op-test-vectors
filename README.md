<h1 align="center">
    <img src="./assets/op-tests.png" alt="OP Test Vectors" width="200px" align="center">
</h1>

<h4 align="center">
    Standard Tests for OP Stack Component Implementations.
</h4>

<p align="center">
  <a href="https://github.com/ethereum-optimism/op-test-vectors/actions/workflows/ci.yml"><img src="https://github.com/ethereum-optimism/op-test-vectors/actions/workflows/ci.yml/badge.svg?label=ci" alt="CI"></a>
  <a href="https://github.com/ethereum-optimism/op-test-vectors/actions/workflows/book.yml"><img src="https://github.com/ethereum-optimism/op-test-vectors/actions/workflows/book.yml/badge.svg?label=Book" alt="Book"></a>
  <img src="https://img.shields.io/badge/License-MIT-green.svg?label=license&labelColor=2a2f35" alt="License">
  <a href="https://ethereum-optimism.github.io/op-test-vectors"><img src="https://img.shields.io/badge/Contributor%20Book-854a15?logo=mdBook&labelColor=2a2f35" alt="Book"></a>
</p>

<p align="center">
  <a href="#whats-op-test-vectors">What's OP Test Vectors?</a> •
  <a href="#overview">Overview</a> •
  <a href="https://static.optimism.io/op-test-vectors/CONTRIBUTING.html">Contributing</a> •
  <a href="#credits">Credits</a>
</p>

## What's OP Test Vectors?

OP Test Vectors is a portable suite of standardized test fixtures used to test OP Stack component implementations.

Test fixtures are static JSON files defined in the [fixtures](./fixtures) directory.

Test fixtures can be easily generated using the [opt8n](./crates/opt8n) cli tool.

### Development Status

`op-test-vectors` is currently in active development, and is not yet ready for use in production.

## Overview

**`op-test-vectors`**

- [`execution`](./crates/op-test-vectors/src/execution.rs): Rust types for the execution test fixtures.
- [`derivation`](./crates/op-test-vectors/src/derivation.rs): Rust types for the derivation test fixtures.

**`opt8n` Commands**

- `repl`: Spins up a REPL that allows the user to send transactions to and generate a test fixture from those transactions.
- `script`: Executes a forge script against an anvil instance and generates the test fixture.

## Book

The [book][book] contains an in-depth overview of the project, contributor guidelines, and tutorials for creating your own test fixtures as well as you own test runners.

## Credits

`op-test-vectors` is inspired by [ethereum/tests][eth-tests] and built by the collaboration between a number of teams and external contributors including [OP Labs][op-labs] and [Worldcoin engineers][worldcoin].

[book]: https://ethereum-optimism.github.io/op-test-vectors/
[op-labs]: https://github.com/ethereum-optimism
[worldcoin]: https://github.com/worldcoin
[eth-tests]: https://github.com/ethereum/tests
