#![allow(dead_code)]

mod type_inference;
mod optimizer;
mod ast;
mod lexer;
mod parser;
mod codegen;
mod verifier;
mod ir;

use std::{
    env, fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process,
    time::Duration,
    thread,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = "brc3";
const REGISTRY_BASE: &str = "https://raw.githubusercontent.com/RaspGrasp/brc3_lib/main";
const LIB_DIR_NAME: &str = "brc3_lib";

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
    fn white(&self, t: &str) -> String { self.paint("1;37", t) }
    fn bg_green(&self, t: &str) -> String { self.paint("42;30", t) }
    fn bg_cyan(&self, t: &str) -> String { self.paint("46;30", t) }
    fn bg_blue(&self, t: &str) -> String { self.paint("44;37", t) }
    fn bright_green(&self, t: &str) -> String { self.paint("92", t) }
    fn bright_cyan(&self, t: &str) -> String { self.paint("96", t) }
    fn bright_magenta(&self, t: &str) -> String { self.paint("95", t) }
}

fn banner(s: &Style) {
    eprintln!();
    eprintln!(
        "  {}  {}",
        s.bold(&s.bright_cyan("━━━")),
        s.bold(&s.white("BedRock Compiler"))
    );
    eprintln!(
        "  {}  {}  ·  multi-target · packages",
        s.dim("│"),
        s.dim(&format!("v{VERSION}"))
    );
    eprintln!("  {}", s.bold(&s.bright_cyan("━━━")));
    eprintln!();
}

fn print_help(s: &Style) {
    banner(s);
    eprintln!(
        "{usage}

{cmd}

    {bin} {file} [OPTIONS]
    {bin} {ins} {pkg}
    {bin} {list}

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

{pkg_sec}

    {ins_ex}
    {list_ex}

{examples}

    {ex1}
    {ex2}
    {ex3}
    {ex4}

{docs}

    Language reference:  https://bedrock.abrdns.com
   
",
        usage = s.bold(&s.bright_cyan("USAGE")),
        cmd = s.dim("Compile a .br source file or manage packages."),
        bin = s.cyan(NAME),
        file = s.yellow("<file.br>"),
        ins = s.green("ins"),
        pkg = s.yellow("<package>"),
        list = s.green("list"),
        targets = s.bold(&s.bright_cyan("TARGETS")),
        t_mips = s.green("mips"),
        t_mle = s.green("mips-le"),
        t_rv = s.green("riscv"),
        t_x86 = s.green("x86"),
        t_ir = s.green("ir"),
        t_arm = s.dim("arm"),
        def = s.dim("(default)"),
        stub = s.yellow("(stub — not implemented)"),
        options = s.bold(&s.bright_cyan("OPTIONS")),
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
        pkg_sec = s.bold(&s.bright_cyan("PACKAGES")),
        ins_ex = s.dim(&format!("{NAME} ins mathutils")),
        list_ex = s.dim(&format!("{NAME} list")),
        examples = s.bold(&s.bright_cyan("EXAMPLES")),
        ex1 = s.dim(&format!("{NAME} app.br --target riscv -o out/app.bin")),
        ex2 = s.dim(&format!("{NAME} app.br --target x86 -o app.elf")),
        ex3 = s.dim(&format!("{NAME} app.br --target ir")),
        ex4 = s.dim(&format!("{NAME} app.br --optimize 2 --target mips")),
        docs = s.bold(&s.bright_cyan("DOCS")),
    );
}

fn print_version(s: &Style) {
    eprintln!(
        "{} {} ({})",
        s.bold(NAME),
        s.bright_cyan(&format!("v{VERSION}")),
        s.dim("GitHub release 3.2")
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
    install_pkg: Option<String>,
    list_pkgs: bool,
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
        install_pkg: None,
        list_pkgs: false,
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
        } else if flag_eq(a, &["ins", "install"]) {
            let v = args
                .get(i + 1)
                .ok_or_else(|| String::from("missing package name for ins"))?;
            cli.install_pkg = Some(v.clone());
            i += 2;
        } else if flag_eq(a, &["list"]) {
            cli.list_pkgs = true;
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

fn lib_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        dirs.push(cwd.join(LIB_DIR_NAME));
    }
    if let Ok(home) = env::var("HOME") {
       dirs.push(PathBuf::from(&home).join(LIB_DIR_NAME));
       dirs.push(PathBuf::from(&home).join(".brc3").join("lib"));
    }
    if let Ok(home) = env::var("USERPROFILE") {
        dirs.push(PathBuf::from(home).join(LIB_DIR_NAME));
    }
    dirs.push(PathBuf::from("/usr/local/lib").join(LIB_DIR_NAME));
    dirs
}

fn ensure_lib_dir(s: &Style) -> PathBuf {
    let preferred = env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(LIB_DIR_NAME);
    if !preferred.exists() {
        if let Err(e) = fs::create_dir_all(&preferred) {
            eprintln!(
                "{}  cannot create {}: {e}",
                s.red("error:"),
                preferred.display()
            );
            process::exit(1);
        }
        eprintln!(
            "{}  created {}",
            s.bright_green("ok:"),
            s.bold(&preferred.display().to_string())
        );
    }
    preferred
}

fn resolve_include(file_name: &str, base_path: &Path) -> Option<PathBuf> {
    let candidates: Vec<String> = if file_name.ends_with(".br") {
        vec![file_name.to_string()]
    } else {
        vec![
            file_name.to_string(),
            format!("{}.br", file_name),
        ]
    };

    for name in &candidates {
        let p = base_path.join(name);
        if p.is_file() {
            return Some(p);
        }
    }

    for dir in lib_dirs() {
        for name in &candidates {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
            let p2 = dir.join(name.trim_end_matches(".br")).join(name);
            if p2.is_file() {
                return Some(p2);
            }
        }
    }
    None
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
                let include_path = match resolve_include(&file_name, base_path) {
                    Some(p) => p,
                    None => {
                        eprintln!(
                            "{}  include file not found: '{}'",
                            s.red("error:"),
                            file_name
                        );
                        eprintln!(
                            "{}  searched: source dir + {}",
                            s.dim("…"),
                            LIB_DIR_NAME
                        );
                        process::exit(1);
                    }
                };
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

fn progress_bar(s: &Style, label: &str, pct: u8) {
    let width = 28usize;
    let filled = ((pct as usize) * width) / 100;
    let empty = width - filled;
    let bar = if s.color {
        let mut out = String::new();
        for i in 0..filled {
            let hue = if i < width / 3 {
                "42"
            } else if i < 2 * width / 3 {
                "46"
            } else {
                "44"
            };
            out.push_str(&format!("\x1b[{}m \x1b[0m", hue));
        }
        out.push_str(&" ".repeat(empty));
        out
    } else {
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    };
    let pct_str = format!("{:>3}%", pct);
    eprint!(
        "\r  {} {} [{}] {}",
        s.bright_cyan("↓"),
        s.bold(label),
        bar,
        s.bright_green(&pct_str)
    );
    let _ = io::stderr().flush();
}

fn download_package(name: &str, s: &Style) {
    let clean = name.trim().trim_end_matches(".br");
    if clean.is_empty() {
        eprintln!("{}  empty package name", s.red("error:"));
        process::exit(1);
    }

    let lib_dir = ensure_lib_dir(s);
    let dest = lib_dir.join(format!("{}.br", clean));

    if dest.exists() {
        eprintln!(
            "{}  package already installed: {}",
            s.yellow("warn:"),
            s.bold(clean)
        );
        eprintln!(
            "{}  path: {}",
            s.dim("…"),
            dest.display()
        );
        return;
    }

    let urls = [
        format!("{}/{}.br", REGISTRY_BASE, clean),
        format!("{}/{}/{}.br", REGISTRY_BASE, clean, clean),
        format!("{}/packages/{}.br", REGISTRY_BASE, clean),
    ];

    eprintln!(
        "{}  installing {}",
        s.blue("info:"),
        s.bold(&s.bright_magenta(clean))
    );

    progress_bar(s, clean, 5);
    thread::sleep(Duration::from_millis(80));

    let mut success = false;
    let mut last_err = String::new();

    for (idx, url) in urls.iter().enumerate() {
        let pct = 10 + (idx as u8) * 20;
        progress_bar(s, clean, pct);

        let tmp = lib_dir.join(format!(".tmp_{}.br", clean));
        let status = process::Command::new("curl")
            .args([
                "-fsSL",
                "--connect-timeout",
                "12",
                "--max-time",
                "60",
                "-o",
                tmp.to_str().unwrap_or("/tmp/brc3_tmp.br"),
                url,
            ])
            .status();

        match status {
            Ok(st) if st.success() => {
                if let Ok(meta) = fs::metadata(&tmp) {
                    if meta.len() > 0 {
                        progress_bar(s, clean, 85);
                        if fs::rename(&tmp, &dest).is_ok() {
                            progress_bar(s, clean, 100);
                            eprintln!();
                            eprintln!(
                                "{}  installed {} → {}",
                                s.bright_green("ok:"),
                                s.bold(clean),
                                s.cyan(&dest.display().to_string())
                            );
                            success = true;
                            break;
                        }
                    }
                }
                let _ = fs::remove_file(&tmp);
                last_err = format!("empty response from {}", url);
            }
            Ok(_) => {
                let _ = fs::remove_file(&tmp);
                last_err = format!("not found: {}", url);
            }
            Err(e) => {
                last_err = format!("curl failed: {e}");
            }
        }
        thread::sleep(Duration::from_millis(60));
    }

    if !success {
        eprintln!();
        eprintln!(
            "{}  failed to install '{}'",
            s.red("error:"),
            clean
        );
        eprintln!("{}  {}", s.dim("…"), last_err);
        eprintln!(
            "{}  registry: {}",
            s.dim("…"),
            REGISTRY_BASE
        );
        process::exit(1);
    }
}

fn list_packages(s: &Style) {
    eprintln!(
        "{}  scanning local packages…",
        s.blue("info:")
    );
    let mut found = false;
    for dir in lib_dirs() {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "br").unwrap_or(false) {
                    if !found {
                        eprintln!();
                        eprintln!(
                            "  {}  {}",
                            s.bold(&s.bright_cyan("PACKAGES")),
                            s.dim(&dir.display().to_string())
                        );
                    }
                    found = true;
                    let name = p
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let size = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    eprintln!(
                        "  {}  {}  {}",
                        s.bright_green("•"),
                        s.bold(&name),
                        s.dim(&format!("({} bytes)", size))
                    );
                }
            }
        }
    }
    if !found {
        eprintln!(
            "{}  no packages installed yet",
            s.yellow("warn:")
        );
        eprintln!(
            "{}  try: {} ins <name>",
            s.dim("…"),
            NAME
        );
    } else {
        eprintln!();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

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

    if let Some(pkg) = &cli.install_pkg {
        download_package(pkg, &s);
        process::exit(0);
    }

    if cli.list_pkgs {
        list_packages(&s);
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
        s.bright_cyan(target.name())
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
        s.bright_cyan(target.name())
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
        s.bright_green("ok:"),
        s.bold(&out_path.display().to_string()),
        binary.len()
    );

    let _map = backend.get_source_map();
}
