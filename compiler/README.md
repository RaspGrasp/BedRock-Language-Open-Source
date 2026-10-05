# BedRock Compiler (`bedrockco`)

**Version:** 1.0.0-foundation  

Multi-target compiler for the [BedRock](https://bedrock.abrdns.com) systems language.  
Pipeline: **source → lexer → parser → types → verifier → IR → backend**.

No external Rust crates. Bare-metal oriented; optional hosted Linux x86-64 ELF.

---

## Install / build

```bash
cargo build --release
# binaries:
#   target/release/bedrockco
#   target/release/brc3        (same entry point)
```

---

## Targets

| `--target` | Output | Status |
|------------|--------|--------|
| `mips` (default) | `.bin` | supported |
| `mips-le` | `.bin` | supported |
| `riscv` | `.bin` | supported (RV32I/M) |
| `x86` | `.elf` | supported (Linux x86-64) |
| `ir` | `.ir` | supported |
| `arm` | — | **stub only** |

---

## Usage

```bash
bedrockco <file.br> [OPTIONS]

  -o, --output <path>     output file (default: <source>.bin|.elf|.ir)
  -t, --target <arch>     mips | mips-le | riscv | x86 | ir
  -O, --optimize <n>      0..3
      --emit-ir           dump IR to stdout
      --bridge            legacy AST→MIPS (mips only)
      --no-color          disable ANSI colors
  -V, --version
  -h, --help
```

### Examples

```bash
bedrockco app.br --target riscv -o out/app.bin
bedrockco app.br --target x86 -o app.elf
bedrockco app.br --target ir
bedrockco app.br --optimize 2 --target mips
bedrockco --version
```

---

## Smoke test

```bash
chmod +x smoke.sh
./smoke.sh
```

---

## Limits (foundation)

- ARM backend not implemented.
- Optimizer is AST-level; treat as experimental.
- Not a drop-in replacement for GCC/LLVM toolchains.
- ESP32 packaging is a separate tool (`bconf`), not this binary.

Docs: https://bedrock.abrdns.com
