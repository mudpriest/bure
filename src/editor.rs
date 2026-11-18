use std::env;
use std::path::Path;
use std::process::Command;

/// Opens `path` in `$VISUAL`, `$EDITOR`, or a platform default and waits for it to exit.
pub fn edit(path: &Path) -> Result<(), String> {
    let cmd = ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(|var| env::var(var).ok())
        .find(|v| !v.trim().is_empty())
        .unwrap_or_else(|| if cfg!(windows) { "notepad" } else { "vi" }.to_string());

    let mut parts = cmd.split_whitespace();
    let program = parts.next().expect("editor command is non-empty");
    let status = Command::new(program)
        .args(parts)
        .arg(path)
        .status()
        .map_err(|e| format!("failed to launch editor `{cmd}`: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("editor `{cmd}` exited with {status}"))
    }
}
