use std::fs;
use std::path::Path;
use std::process;

/// Renames entries inside `dir` in two phases (source -> temp -> target) so swaps and cycles work.
pub fn apply(dir: &Path, renames: &[(String, String)]) -> Result<(), String> {
    let pid = process::id();
    let temps: Vec<_> = (0..renames.len())
        .map(|i| dir.join(format!(".bure-tmp-{pid}-{i}")))
        .collect();

    for ((old, _), tmp) in renames.iter().zip(&temps) {
        fs::rename(dir.join(old), tmp).map_err(|e| format!("failed to move {old:?}: {e}"))?;
    }

    for ((old, new), tmp) in renames.iter().zip(&temps) {
        let target = dir.join(new);
        if target.exists() {
            return Err(format!(
                "{new:?} already exists; {old:?} was left as {}",
                tmp.display()
            ));
        }
        fs::rename(tmp, &target).map_err(|e| {
            format!("failed to rename {old:?} -> {new:?} (left as {}): {e}", tmp.display())
        })?;
    }

    Ok(())
}
