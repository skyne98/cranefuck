# Cranefuck

[![License](https://img.shields.io/github/license/skyne98/cranefuck?style=flat-square)](https://github.com/skyne98/cranefuck/blob/master/LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/skyne98/cranefuck/ci.yml?style=flat-square)](https://github.com/skyne98/cranefuck/actions)
[![Cranelift](https://img.shields.io/badge/JIT-Cranelift-blue?style=flat-square)](https://github.com/bytecodealliance/wasmtime/tree/master/cranelift)

🚧 **PROTOTYPE – NOT PRODUCTION READY!** 🚧\
_(But let's be honest: Brainfuck is never ready for production.)_

Cranefuck is a high-performance **Just-In-Time (JIT) Brainfuck runtime** powered
by
[Cranelift](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift).
Designed with speed, simplicity, and elegance in mind, this project is the
brainchild of a single developer.

## Features

- 🚀 **High-Speed JIT Compilation** powered by Cranelift
- 🧩 **Minimalist Design** for maximum efficiency
- 🖥️ **Cross-Platform Compatibility** (currently tested on **Windows**)
- 📦 **Lightweight Dependencies** without sacrificing performance
- 🛠️ **Optimized Execution Pipeline** for running Brainfuck programs

## Installation

### Via Cargo

Install directly from GitHub:

```sh
cargo install --git https://github.com/skyne98/cranefuck
```

### Building from Source

Clone the repository and build the release version:

```sh
git clone https://github.com/skyne98/cranefuck.git
cd cranefuck
cargo build --release
```

## Usage

### Running a Brainfuck File

Execute a Brainfuck program from a file:

```sh
cranefuck examples/hello.bf
```

### Running from Standard Input

Pipe Brainfuck code directly into Cranefuck:

```sh
echo "+[----->+++<]>++." | cranefuck
```

## Contributing

🚨 **FEEDBACK WANTED!** 🚨

As a solo developer, I welcome you guys to turn this project into a fun
community playground for performant brainfuck! Please share your feedback and
submit PRs! [GitHub](https://github.com/skyne98/cranefuck).

## License

This project is distributed under the [MIT License](LICENSE).

---

Made with ❤️ by [skyne98](https://github.com/skyne98)
