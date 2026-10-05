# BedRock Programming Language

BedRock is a low-level systems language for direct hardware control and OS-style work.
High-level logic maps to fixed machine sequences with no hidden runtime and no leftover abstraction between your source and the binary.

## Core philosophy

Three rules drive the design.

No Magic. What you write is what runs. No GC no implicit allocations no virtual dispatch by default.

Byte-Level Validation. Output is raw words you can audit with a hex view or objdump.

Direct Hardware Control. poke peek outb inb and asm reach memory and devices without an OS layer in the middle.

## What you get

A compiler from scratch in Rust with no LLVM middle end and no external crates in the foundation cut.

Pipeline source to lexer to parser to types to verifier to IR to backend.

Targets in the current foundation tree

mips and mips-le raw binary  
riscv RV32I/M raw binary  
x86 Linux x86-64 ELF  
ir text dump of BedRock-IR  
arm listed as a stub only

Driver binary is brc3 same entry as bedrockco or bedrock when those aliases are built.

Docs for the language surface live at https://bedrock.abrdns.com


## Compiler pipeline

```text
.source.br
    │
    ▼
┌─────────┐
│  Lexer  │  tokens + includes
└────┬────┘
     ▼
┌─────────┐
│ Parser  │  AST
└────┬────┘
     ▼
┌────────────────┐
│ Type inference │
└────┬───────────┘
     ▼
┌──────────┐
│ Verifier │
└────┬─────┘
     ▼
┌─────────────────────┐
│ Optimizer optional  │  -O 1..3
└────┬────────────────┘
     ▼
┌────────────┐
│ IR builder │  main entry when main exists
└────┬───────┘
     ▼
┌──────────────────────────────┐
│ Backend                      │
│ mips │ mips-le │ riscv │ x86 │ ir
└──────────────────────────────┘
     ▼
.bin  .elf  or  .ir
```

One IR shared across targets. Raw asm hex stays architecture specific.

## Project timeline

Initial development of architecture and compiler logic started December 2025.

Core engine work through early 2026 covered parsing typing IR and stress style checks on logic memory calls and register pressure.

v2.3 added a real RISC-V backend beside MIPS and pushed past compile-only checks into hardware-facing validation including ESP-IDF style images and simulator runs on an RV core.

v3 foundation cut hardened stack and call behaviour under spill pressure added hosted return into a C host when requested added an x86-64 ELF path and cleaned the CLI so -o version and targets behave in scripts.

## Current status

Phase multi-target compiler foundation plus early ecosystem tooling around packaging and boards.

Language and IR direction remain the centre of the repo. Flash image helpers and board demos sit beside the compiler not inside every release binary claim.

## Roadmap

Keep backends honest under composition nested calls spilling hosted vs freestanding.

Grow host path usefulness for subset self-hosting on Linux x86.

Expand hardware-facing libraries and examples without pretending a full OS exists yet.

ARM stays out of the supported set until it is real.

## Build and run

```bash
cargo build --release

./path/to/brc3 --version
./path/to/brc3 app.br --target riscv -o out/app.bin
./path/to/brc3 app.br --target x86 -o app.elf
./path/to/brc3 app.br --target ir
```

## Links

Language documentation https://bedrock.abrdns.com

VS Code Marketplace publisher https://marketplace.visualstudio.com/publishers/mrDevRussia

VS Code BedRock extension https://marketplace.visualstudio.com/items?itemName=mrDevRussia.bedrock-lang

## Credit and scope

Compiler core lexer parser typing IR optimizers and the long MIPS line are original project work. Later backend and tooling passes used assistants for review and grunt work with testing and ownership kept on the author side.

This README describes the language and the foundation compiler. It does not claim every peripheral demo on every MCU is complete.
