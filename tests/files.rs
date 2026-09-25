use htmlp::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "htmlp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Zero;
impl TokenCounter for Zero {
    fn name(&self) -> &str {
        "cl100k_base"
    }
    fn count(&self, _: &str) -> u64 {
        0
    }
}
const VALID: &str = "<htmlp max-tokens='1k' reason='Shared context.'>Review.</htmlp>";
#[test]
fn scans_independently_and_ignores_configs_and_generated_files() {
    let dir = Temp::new();
    fs::create_dir(dir.0.join("nested")).unwrap();
    fs::create_dir(dir.0.join("target")).unwrap();
    fs::write(dir.0.join("a.htmlp"), VALID).unwrap();
    fs::write(dir.0.join("nested/b.htmlp"), VALID).unwrap();
    fs::write(dir.0.join(".htmlp.json"), "not valid JSON").unwrap();
    fs::write(dir.0.join("target/bad.htmlp"), "broken").unwrap();
    let reports = check_path(&dir.0, &Zero).unwrap();
    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|r| r.report.is_ok()));
    assert!(reports[0].file.ends_with("a.htmlp"));
}
#[test]
fn no_matches_and_invalid_utf8_fail() {
    let dir = Temp::new();
    assert!(check_path(&dir.0, &Zero).is_err());
    fs::write(dir.0.join("bad.htmlp"), [0xff]).unwrap();
    assert!(!check_path(&dir.0, &Zero).unwrap()[0].report.is_ok());
}
#[cfg(unix)]
#[test]
fn directory_symlinks_are_not_followed() {
    let dir = Temp::new();
    fs::write(dir.0.join("a.htmlp"), VALID).unwrap();
    std::os::unix::fs::symlink(&dir.0, dir.0.join("loop")).unwrap();
    assert_eq!(check_path(&dir.0, &Zero).unwrap().len(), 1);
}
#[cfg(feature = "cli")]
#[test]
fn cli_output_and_exit_codes() {
    use std::process::Command;
    let dir = Temp::new();
    let file = dir.0.join("prompt.htmlp");
    fs::write(&file, VALID).unwrap();
    let run = |command: &str| {
        Command::new(env!("CARGO_BIN_EXE_htmlp"))
            .arg(command)
            .arg(&file)
            .output()
            .unwrap()
    };
    let result = run("compile");
    assert!(result.status.success());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["limits"]["reason"], "Shared context.");
    assert_eq!(run("render").stdout, b"Review.");
    fs::write(&file, "<htmlp max-tokens='1k'></htmlp>").unwrap();
    let result = run("check");
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("error reason:")
    );
    assert_eq!(run("nonsense").status.code(), Some(2));
}
