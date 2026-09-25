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
    use std::io::Read;
    let path = path.as_ref();
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
    parse(&source)
}

/// Recursively check `.htmlp` files independently, in sorted order. Skip symlinks
/// and `.git`, `node_modules`, `target`, `dist`, and `vendor`. An explicit file
/// must have `.htmlp` extension. No matches is an error, avoiding empty CI passes.
pub fn check_path(
    path: impl AsRef<Path>,
    counter: &impl TokenCounter,
) -> Result<Vec<FileReport>, String> {
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
    visit(path.as_ref(), &mut paths, 0)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "No .htmlp files found at {}",
            path.as_ref().display()
        ));
    }
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
