//! BedRock Compiler — CLI entry point
//! Foundation release: multi-target IR pipeline (MIPS / RISC-V / x86-64).

// Backend stubs and register pools intentionally retain unused items.
#![allow(dead_code)]

mod type_inference;
mod optimizer;
mod ast;
mod lexer;
mod parser;
mod codegen;
mod verifier;
mod ir;

use std::{env, fs, io::IsTerminal, path::{Path, PathBuf}, process};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = "bedrockco";

// ── ANSI styling (no external crates) ─────────────────────

struct Style {
    color: bool,
}

impl Style {
    fn new(force_off: bool) -> Self {
        let color = !force_off
            && env::var_os("NO_COLOR").is_none()
            && std::io::stderr().is_terminal();
        Style { color }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn bold(&self, t: &str) -> String { self.paint("1", t) }
    fn dim(&self, t: &str) -> String { self.paint("2", t) }
    fn red(&self, t: &str) -> String { self.paint("1;31", t) }
    fn green(&self, t: &str) -> String { self.paint("1;32", t) }
    fn yellow(&self, t: &str) -> String { self.paint("1;33", t) }
    fn blue(&self, t: &str) -> String { self.paint("1;34", t) }
    fn magenta(&self, t: &str) -> String { self.paint("1;35", t) }
    fn cyan(&self, t: &str) -> String { self.paint("1;36", t) }
}

fn banner(s: &Style) {
    eprintln!();
    eprintln!(
        "  {}  {}",
        s.bold(&s.cyan("━━━")),
        s.bold("BedRock Compiler")
    );
    eprintln!(
        "  {}  {}  ·  multi-target · bare-metal",
        s.dim("│"),
        s.dim(&format!("v{VERSION}"))
    );
    eprintln!("  {}", s.bold(&s.cyan("━━━")));
    eprintln!();
}

fn print_help(s: &Style) {
    banner(s);
    eprintln!(
        "{usage}

{cmd}

    {bin} {file} [OPTIONS]

{targets}

    {t_mips}     MIPS-32 big-endian raw binary      {def}
    {t_mle}  MIPS-32 little-endian raw binary
    {t_rv}    RISC-V RV32I/M raw binary
    {t_x86}      x86-64 Linux ELF executable
    {t_ir}       emit BedRock-IR text (no machine code)
    {t_arm}      ARM                    {stub}

{options}

    {o_o} {path}       write output to {path} (default: <source>.{{bin|elf|ir}})
    {o_t} {arch}  codegen target (default: mips)
    {o_opt} {n}   optimization level 0..3 (default: 0)
    {o_ir}            print IR to stdout and exit
    {o_br}            legacy AST→MIPS path (mips targets only)
    {o_nc}          disable ANSI colors
    {o_v}         print version and exit
    {o_h}            show this help

{examples}

    {ex1}
    {ex2}
    {ex3}
    {ex4}

{docs}

    Language reference:  https://bedrock.abrdns.com
",
        usage = s.bold("USAGE"),
        cmd = s.dim("Compile a .br source file to machine code or IR."),
        bin = s.cyan(NAME),
        file = s.yellow("<file.br>"),
        targets = s.bold("TARGETS"),
        t_mips = s.green("mips"),
        t_mle = s.green("mips-le"),
        t_rv = s.green("riscv"),
        t_x86 = s.green("x86"),
        t_ir = s.green("ir"),
        t_arm = s.dim("arm"),
        def = s.dim("(default)"),
        stub = s.yellow("(stub — not implemented)"),
        options = s.bold("OPTIONS"),
        o_o = s.cyan("-o"),
        path = s.yellow("<path>"),
        o_t = s.cyan("--target"),
        arch = s.yellow("<arch>"),
        o_opt = s.cyan("--optimize"),
        n = s.yellow("<n>"),
        o_ir = s.cyan("--emit-ir"),
        o_br = s.cyan("--bridge"),
        o_nc = s.cyan("--no-color"),
        o_v = s.cyan("--version"),
        o_h = s.cyan("--help"),
        examples = s.bold("EXAMPLES"),
        ex1 = s.dim(&format!("{NAME} app.br --target riscv -o out/app.bin")),
        ex2 = s.dim(&format!("{NAME} app.br --target x86 -o app.elf")),
        ex3 = s.dim(&format!("{NAME} app.br --target ir")),
        ex4 = s.dim(&format!("{NAME} app.br --optimize 2 --target mips")),
        docs = s.bold("DOCS"),
    );
}

fn print_version(s: &Style) {
    eprintln!(
        "{} {} ({})",
        s.bold(NAME),
        s.cyan(&format!("v{VERSION}")),
        s.dim("foundation")
    );
    eprintln!(
        "{}",
        s.dim("targets: mips, mips-le, riscv, x86, ir  |  arm: stub")
    );
}

fn flag_eq(a: &str, names: &[&str]) -> bool {
    names.iter().any(|n| a == *n)
}

struct Cli {
    source: Option<PathBuf>,
    output: Option<PathBuf>,
    target: String,
    opt_level: u8,
    emit_ir: bool,
    use_bridge: bool,
    no_color: bool,
    want_help: bool,
    want_version: bool,
}

fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli {
        source: None,
        output: None,
        target: "mips".into(),
        opt_level: 0,
        emit_ir: false,
        use_bridge: false,
        no_color: false,
        want_help: false,
        want_version: false,
    };

    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if flag_eq(a, &["--help", "-h"]) {
            cli.want_help = true;
            i += 1;
        } else if flag_eq(a, &["--version", "-V"]) {
            cli.want_version = true;
            i += 1;
        } else if flag_eq(a, &["--no-color"]) {
            cli.no_color = true;
            i += 1;
        } else if flag_eq(a, &["--emit-ir"]) {
            cli.emit_ir = true;
            i += 1;
        } else if flag_eq(a, &["--bridge"]) {
            cli.use_bridge = true;
            i += 1;
        } else if flag_eq(a, &["--target", "-t"]) {
            let v = args
                .get(i + 1)
                .ok_or_else(|| String::from("missing value for --target"))?;
            cli.target = v.clone();
            i += 2;
        } else if flag_eq(a, &["--optimize", "-O"]) {
            let v = args
                .get(i + 1)
                .ok_or_else(|| String::from("missing value for --optimize"))?;
            cli.opt_level = v
                .parse()
                .map_err(|_| format!("invalid --optimize value '{v}' (expected 0..3)"))?;
            if cli.opt_level > 3 {
                return Err(String::from("--optimize level must be 0..3"));
            }
            i += 2;
        } else if flag_eq(a, &["-o", "--output"]) {
            let v = args
                .get(i + 1)
                .ok_or_else(|| String::from("missing value for -o"))?;
            cli.output = Some(PathBuf::from(v));
            i += 2;
        } else if a.starts_with('-') {
            return Err(format!("unknown option '{a}' (try --help)"));
        } else if cli.source.is_none() {
            cli.source = Some(PathBuf::from(a));
            i += 1;
        } else {
            return Err(format!("unexpected argument '{a}'"));
        }
    }
    Ok(cli)
}

fn process_includes(
    content: String,
    base_path: &Path,
    included: &mut std::collections::HashSet<String>,
    stack: &mut Vec<String>,
    s: &Style,
) -> String {
    let mut final_code = String::new();
    for line in content.lines() {
        if line.trim().starts_with("include ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                let file_name = parts[1].replace('\"', "").replace(';', "");
                let include_path = base_path.join(&file_name);
                let canonical = match include_path.canonicalize() {
                    Ok(p) => p.to_string_lossy().to_string(),
                    Err(_) => include_path.to_string_lossy().to_string(),
                };
                if stack.contains(&canonical) {
                    let chain: Vec<String> = stack
                        .iter()
                        .map(|p| {
                            Path::new(p)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        })
                        .collect();
                    eprintln!(
                        "{}  Circular include!\n         chain: {} -> {}",
                        s.red("error:"),
                        chain.join(" -> "),
                        file_name
                    );
                    process::exit(1);
                }
                if included.contains(&canonical) {
                    continue;
                }
                included.insert(canonical.clone());
                stack.push(canonical.clone());
                let include_content = match fs::read_to_string(&include_path) {
                    Ok(c) => c,
                    Err(_) => {
                        eprintln!(
                            "{}  include file not found: '{}'\n         looked in: {}",
                            s.red("error:"),
                            file_name,
                            include_path.display()
                        );
                        process::exit(1);
                    }
                };
                final_code.push_str(&process_includes(
                    include_content,
                    include_path.parent().unwrap_or(base_path),
                    included,
                    stack,
                    s,
                ));
                final_code.push('\n');
                stack.pop();
            }
        } else {
            final_code.push_str(line);
            final_code.push('\n');
        }
    }
    final_code
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Pre-scan for --no-color before any output
    let no_color_early = args.iter().any(|a| a == "--no-color");
    let s = Style::new(no_color_early);

    if args.len() < 2 {
        print_help(&s);
        process::exit(0);
    }

    let cli = match parse_cli(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}  {e}", s.red("error:"));
            eprintln!("{}", s.dim(&format!("run `{NAME} --help` for usage")));
            process::exit(2);
        }
    };

    if cli.want_help {
        print_help(&s);
        process::exit(0);
    }
    if cli.want_version {
        print_version(&s);
        process::exit(0);
    }

    let source_path = match &cli.source {
        Some(p) => p.clone(),
        None => {
            eprintln!("{}  missing source file (.br)", s.red("error:"));
            eprintln!("{}", s.dim(&format!("usage: {NAME} <file.br> [OPTIONS]")));
            process::exit(2);
        }
    };

    let target = codegen::Target::from_str(&cli.target);
    if matches!(target, codegen::Target::Arm) {
        eprintln!(
            "{}  ARM backend is a {} — no code generated",
            s.yellow("warning:"),
            s.yellow("stub")
        );
        eprintln!(
            "{}",
            s.dim("         use --target mips | riscv | x86 | ir")
        );
    }

    let use_bridge = cli.use_bridge;
    let emit_ir = cli.emit_ir;
    let opt_level = cli.opt_level;

    if use_bridge && target != codegen::Target::Mips && target != codegen::Target::MipsLe {
        eprintln!(
            "{}  --bridge is only available with --target mips or mips-le",
            s.red("error:")
        );
        process::exit(1);
    }

    let source_code = match fs::read_to_string(&source_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "{}  source not found: {}",
                s.red("error:"),
                s.bold(&source_path.display().to_string())
            );
            process::exit(1);
        }
    };

    eprintln!(
        "{}  compiling {}  →  {}",
        s.blue("info:"),
        s.bold(&source_path.display().to_string()),
        s.cyan(target.name())
    );

    let base_path = source_path.parent().unwrap_or(Path::new("."));
    let mut included = std::collections::HashSet::new();
    let mut stack = Vec::new();
    let processed = process_includes(source_code, base_path, &mut included, &mut stack, &s);

    let mut lexer = lexer::Lexer::new(&processed);
    let mut tokens = Vec::new();
    let mut locs: Vec<(usize, usize)> = Vec::new();
    loop {
        let line = lexer.line();
        let col = lexer.col();
        let tok = lexer.next_token();
        if tok == lexer::Token::EOF {
            break;
        }
        tokens.push(tok);
        locs.push((line, col));
    }

    eprintln!(
        "{}  lexed {} tokens",
        s.dim("…"),
        s.dim(&tokens.len().to_string())
    );

    let mut parser = parser::Parser::new_with_locs(tokens, locs);
    let program = parser.parse_program();

    let mut inferencer = type_inference::TypeInferencer::new();
    let program = inferencer.run(program);
    let mut ver = verifier::Verifier::new();
    ver.run(&program);

    let program = match opt_level {
        1 => {
            eprintln!("{}  optimize L1 (pruner)", s.dim("…"));
            optimizer::pruner::Pruner::new().run(program)
        }
        2 => {
            eprintln!("{}  optimize L2 (pruner + transformer)", s.dim("…"));
            let p = optimizer::pruner::Pruner::new().run(program);
            optimizer::transformer::Transformer::new().run(p)
        }
        3 => {
            eprintln!("{}  optimize L3 (full)", s.dim("…"));
            optimizer::Optimizer::new().run(program)
        }
        _ => program,
    };

    if use_bridge {
        eprintln!(
            "{}  mode {}  (AST → MIPS direct)",
            s.blue("info:"),
            s.magenta("BRIDGE")
        );
        let mut cg = codegen::mips::LegacyCodegen::new();
        let binary = cg.compile(&program);
        let out_path = cli
            .output
            .clone()
            .unwrap_or_else(|| source_path.with_extension("bin"));
        if let Err(e) = fs::write(&out_path, &binary) {
            eprintln!("{}  write failed: {e}", s.red("error:"));
            process::exit(1);
        }
        eprintln!(
            "{}  wrote {}  ({} bytes)",
            s.green("ok:"),
            s.bold(&out_path.display().to_string()),
            binary.len()
        );
        return;
    }

    eprintln!(
        "{}  mode {}  target={}",
        s.blue("info:"),
        s.magenta("IR pipeline"),
        s.cyan(target.name())
    );

    let mut ir_builder = ir::builder::IrBuilder::new();
    let ir_module = ir_builder.build(program);

    if emit_ir {
        ir_module.dump();
        eprintln!("{}  IR dumped to stdout", s.green("ok:"));
        return;
    }

    let mut backend = codegen::select_backend(&target);
    let binary = backend.compile(&ir_module);

    let out_ext = target.output_extension();
    let out_path = cli
        .output
        .clone()
        .unwrap_or_else(|| source_path.with_extension(out_ext));

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    if let Err(e) = fs::write(&out_path, &binary) {
        eprintln!("{}  write failed: {e}", s.red("error:"));
        process::exit(1);
    }

    eprintln!(
        "{}  wrote {}  ({} bytes)",
        s.green("ok:"),
        s.bold(&out_path.display().to_string()),
        binary.len()
    );

    let _map = backend.get_source_map();
}
