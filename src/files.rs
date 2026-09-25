use crate::{Diagnostic, Document, Position, Report, TokenCounter, lint, parse};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A per-file report; files in a directory are never merged or inherited.
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct FileReport {
    pub file: PathBuf,
    pub report: Report,
}

/// Read UTF-8 source with a 4 MiB cap before allocating the full input.
pub fn parse_file(path: impl AsRef<Path>) -> Result<Document, Diagnostic> {
    parse(&read_source(path.as_ref())?)
}

fn read_source(path: &Path) -> Result<String, Diagnostic> {
    use std::io::Read;
    let failure = |e: std::io::Error| {
        Diagnostic::new(
            "io",
            format!("{}: {e}", path.display()),
            Position::default(),
        )
    };
    let file = fs::File::open(path).map_err(failure)?;
    let mut source = String::new();
    file.take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut source)
        .map_err(failure)?;
    Ok(source)
}

/// Recursively check `.htmlp` files independently, in sorted order. Skip symlinks
/// and `.git`, `node_modules`, `target`, `dist`, and `vendor`. An explicit file
/// must have `.htmlp` extension. No matches is an error, avoiding empty CI passes.
pub fn check_path(
    path: impl AsRef<Path>,
    counter: &impl TokenCounter,
) -> Result<Vec<FileReport>, String> {
    let paths = source_paths(path.as_ref())?;
    Ok(paths
        .into_iter()
        .map(|file| {
            let report = match parse_file(&file) {
                Ok(doc) => lint(&doc, counter),
                Err(error) => Report {
                    diagnostics: vec![error],
                    measurements: Vec::new(),
                },
            };
            FileReport { file, report }
        })
        .collect())
}

fn source_paths(path: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(path: &Path, files: &mut Vec<PathBuf>, depth: usize) -> Result<(), String> {
        if depth > 128 {
            return Err("Directory nesting exceeds 128 levels".into());
        }
        let metadata =
            fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Ok(());
        }
        if metadata.is_file() {
            if path.extension().is_some_and(|e| e == "htmlp") {
                files.push(path.to_path_buf());
            }
        } else if metadata.is_dir() {
            let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
            for entry in entries {
                let entry = entry.map_err(|e| e.to_string())?;
                if matches!(
                    entry.file_name().to_str(),
                    Some(".git" | "node_modules" | "target" | "dist" | "vendor")
                ) {
                    continue;
                }
                visit(&entry.path(), files, depth + 1)?;
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    visit(path, &mut paths, 0)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!("No .htmlp files found at {}", path.display()));
    }
    Ok(paths)
}

/// Sign all `.htmlp` files using the same traversal as [`check_path`]. All files
/// are parsed before any writes. Changed files are replaced atomically, keeping
/// permissions. Returns changed paths. An I/O failure can leave earlier files
/// updated; a directory is not a transaction. Avoid concurrent source edits.
pub fn sign_path(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, String> {
    let mut changes = Vec::new();
    for file in source_paths(path.as_ref())? {
        let before = read_source(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let after = crate::sign_source(&before).map_err(|e| format!("{}: {e}", file.display()))?;
        if before != after {
            changes.push((file, before, after));
        }
    }
    let mut changed = Vec::new();
    for (file, before, after) in changes {
        replace_source(&file, &before, &after).map_err(|e| format!("{}: {e}", file.display()))?;
        changed.push(file);
    }
    Ok(changed)
}

fn replace_source(path: &Path, before: &str, after: &str) -> std::io::Result<()> {
    use std::io::{Error, Write};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::other("Source is no longer a regular file"));
    }
    let temporary = path.with_file_name(format!(
        ".htmlp-sign-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        output.write_all(after.as_bytes())?;
        output.set_permissions(metadata.permissions())?;
        output.sync_all()?;
        drop(output);
        if read_source(path).map_err(Error::other)? != before {
            return Err(Error::other(
                "Source changed while signing; rerun htmlp sign",
            ));
        }
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
