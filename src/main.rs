use htmlp::{Bindings, Cl100k, Diagnostic, check_path, lint, parse_file, render, sign_path};
use std::{env, fs, path::Path, process::ExitCode};
const HELP: &str = "HTMLP — Token limits for prompt files.\n\nhtmlp sign PATH\nhtmlp check PATH [--json]\nhtmlp parse FILE\nhtmlp compile FILE\nhtmlp render FILE [--vars bindings.json]\nhtmlp watch PATH\n\ncheck recursively validates .htmlp files independently. No repository config.\ncompile checks static budgets and emits JSON; variable-dependent budgets require render.\nExit: 0 success, 1 file validation errors, 2 usage or operational errors.\n";
fn emit(file: &str, diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        eprintln!(
            "{file}:{}:{}: error {}: {}",
            d.position.line.max(1),
            d.position.column.max(1),
            d.code,
            d.message.replace('\n', " ")
        );
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("htmlp: {error}");
            ExitCode::from(2)
        }
    }
}
fn run() -> Result<u8, String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print!("{HELP}");
        return Ok(0);
    };
    if command == "--help" || command == "-h" {
        print!("{HELP}");
        return Ok(0);
    }
    if command == "--version" {
        println!("htmlp {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    if !["sign", "check", "parse", "compile", "render", "watch"].contains(&command.as_str()) {
        return Err(HELP.into());
    }
    let file = args.next().ok_or(HELP)?;
    let mut json = false;
    let mut vars = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--json" if command == "check" => json = true,
            "--vars" if command == "render" => {
                vars = Some(args.next().ok_or("--vars needs a JSON file")?)
            }
            _ => return Err(format!("Unknown option: {flag}")),
        }
    }
    if command == "sign" {
        let changed = sign_path(&file)?;
        println!("HTMLP signed: {} file(s) updated", changed.len());
        return Ok(0);
    }
    if command == "parse" {
        return match parse_file(&file) {
            Ok(doc) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?
                );
                Ok(0)
            }
            Err(e) => {
                emit(&file, &[e]);
                Ok(1)
            }
        };
    }
    let counter = Cl100k::new()?;
    if command == "watch" {
        // Content snapshots detect atomic saves and added/deleted files without
        // a watcher dependency. Polling is limited to the explicit tree.
        fn snapshot(
            path: &Path,
            depth: usize,
            out: &mut Vec<(std::path::PathBuf, Vec<u8>)>,
        ) -> Result<(), String> {
            if depth > 128 {
                return Err("Directory nesting exceeds 128 levels".into());
            }
            let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() {
                return Ok(());
            }
            if meta.is_dir() {
                for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if matches!(
                        entry.file_name().to_str(),
                        Some(".git" | "node_modules" | "target" | "dist" | "vendor")
                    ) {
                        continue;
                    }
                    snapshot(&entry.path(), depth + 1, out)?;
                }
            } else if path.extension().is_some_and(|e| e == "htmlp") {
                use std::io::Read;
                let mut bytes = Vec::new();
                fs::File::open(path)
                    .map_err(|e| e.to_string())?
                    .take(4 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                out.push((path.to_path_buf(), bytes));
            }
            Ok(())
        }
        let mut previous = None;
        loop {
            let mut state = Vec::new();
            let error = snapshot(Path::new(&file), 0, &mut state).err();
            state.sort_by(|a, b| a.0.cmp(&b.0));
            let current = (state, error);
            if previous.as_ref() != Some(&current) {
                println!("HTMLP checking");
                match check_path(&file, &counter) {
                    Ok(reports) => {
                        for report in reports {
                            emit(
                                &report.file.display().to_string(),
                                &report.report.diagnostics,
                            );
                        }
                    }
                    Err(e) => eprintln!("{file}:1:1: error io: {e}"),
                }
                println!("HTMLP ready");
                previous = Some(current);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    if command == "check" {
        let reports = check_path(&file, &counter)?;
        let failed = reports.iter().any(|r| !r.report.is_ok());
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&reports).map_err(|e| e.to_string())?
            );
        } else {
            for r in &reports {
                emit(&r.file.display().to_string(), &r.report.diagnostics);
            }
            if !failed {
                let deferred = reports
                    .iter()
                    .flat_map(|r| &r.report.measurements)
                    .filter(|m| m.deferred && m.limit.is_some())
                    .count();
                if deferred == 0 {
                    println!("HTMLP OK: {} file(s)", reports.len());
                } else {
                    println!(
                        "HTMLP OK: {} file(s); {deferred} budget(s) deferred until render",
                        reports.len()
                    );
                }
            }
        }
        return Ok(u8::from(failed));
    }
    let doc = match parse_file(&file) {
        Ok(doc) => doc,
        Err(e) => {
            emit(&file, &[e]);
            return Ok(1);
        }
    };
    if command == "compile" {
        let report = lint(&doc, &counter);
        if !report.is_ok() {
            emit(&file, &report.diagnostics);
            return Ok(1);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?
        );
        return Ok(0);
    }
    let bindings: Bindings = match vars {
        Some(path) => serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Expected a JSON object of string bindings: {e}"))?,
        None => Bindings::new(),
    };
    match render(&doc, &bindings, &counter) {
        Ok(text) => {
            print!("{text}");
            Ok(0)
        }
        Err(errors) => {
            emit(&file, &errors);
            Ok(1)
        }
    }
}
