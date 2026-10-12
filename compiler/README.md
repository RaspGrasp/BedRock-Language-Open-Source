# BedRock Compiler (`brc3`)

**Version:** 1.0.0-foundation + packages

Multi-target compiler for the BedRock systems language.
Pipeline: **source → lexer → parser → types → verifier → IR → backend**.

No external Rust crates. Bare-metal oriented; optional hosted Linux x86-64 ELF.

---

## Install / build

```bash
cargo build --release
# binary: target/release/brc3
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
brc3 <file.br> [OPTIONS]
brc3 ins <package>
brc3 list

  -o, --output <path>     output file (default: <source>.bin|.elf|.ir)
  -t, --target <arch>     mips | mips-le | riscv | x86 | ir
  -O, --optimize <n>      0..3
      --emit-ir           dump IR to stdout
      --bridge            legacy AST→MIPS (mips only)
      --no-color          disable ANSI colors
  -V, --version
  -h, --help
```

### Packages

```bash
brc3 ins mathutils
brc3 list
```

Installed packages land in `./brc3_lib/` (created automatically).
`include "mathutils";` or `include "mathutils.br";` resolves from:
1. directory of the source file
2. `./brc3_lib/`
3. `$HOME/brc3_lib` and `$HOME/.brc3/lib`
4. `/usr/local/lib/brc3_lib`

Registry URL (edit in `src/main.rs` → `REGISTRY_BASE`):
`https://raw.githubusercontent.com/bedrock-lang/brc3-packages/main`

### Examples

```bash
brc3 app.br --target riscv -o out/app.bin
brc3 app.br --target x86 -o app.elf
brc3 app.br --target ir
brc3 app.br --optimize 2 --target mips
brc3 ins kerlib
```

---

## Package website

See `packages-site/index.html` — static front-end that lists every `.br` file from your GitHub packages repo via the GitHub API.

---

## Limits (foundation)

- ARM backend not implemented.
- Optimizer is AST-level; treat as experimental.
- Download uses `curl` (must be on PATH).
- Not a drop-in replacement for GCC/LLVM toolchains.

Docs: https://bedrock.abrdns.com
