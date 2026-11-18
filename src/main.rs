mod editor;
mod plan;
mod rename;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bure: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let dir = PathBuf::from(env::args().nth(1).unwrap_or_else(|| ".".into()));
    let old = list_entries(&dir)?;
    if old.is_empty() {
        println!("{} is empty", dir.display());
        return Ok(());
    }

    let tmp = env::temp_dir().join(format!("bure-{}.txt", std::process::id()));
    let edited = edit_names(&tmp, &old);
    let _ = fs::remove_file(&tmp);
    let renames = plan::plan(&old, &edited?).map_err(|e| e.to_string())?;

    if renames.is_empty() {
        println!("Nothing to rename");
        return Ok(());
    }
    for (o, n) in &renames {
        let slash = if dir.join(o).is_dir() { "/" } else { "" };
        println!("{o}{slash} -> {n}{slash}");
    }
    if !confirm(&format!("Apply {} renames? [y/N] ", renames.len()))? {
        println!("Aborted");
        return Ok(());
    }

    rename::apply(&dir, &renames)?;
    println!("Renamed {} entries", renames.len());
    Ok(())
}

/// Lists entry names, directories first and marked with a trailing `/`, like `ls -p --group-directories-first`.
fn list_entries(dir: &Path) -> Result<Vec<String>, String> {
    let read = fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut entries = read
        .map(|entry| {
            let entry = entry.map_err(|e| e.to_string())?;
            let is_dir = entry.path().is_dir();
            let name = entry
                .file_name()
                .into_string()
                .map_err(|n| format!("non-UTF-8 file name: {}", n.to_string_lossy()))?;
            Ok((is_dir, name))
        })
        .collect::<Result<Vec<_>, String>>()?;
    entries.sort_by(|(a_dir, a), (b_dir, b)| {
        b_dir
            .cmp(a_dir)
            .then_with(|| a.to_lowercase().cmp(&b.to_lowercase()))
            .then_with(|| a.cmp(b))
    });
    Ok(entries
        .into_iter()
        .map(|(is_dir, name)| if is_dir { name + "/" } else { name })
        .collect())
}

fn edit_names(tmp: &Path, names: &[String]) -> Result<String, String> {
    let mut content = names.join("\n");
    content.push('\n');
    fs::write(tmp, content).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    editor::edit(tmp)?;
    fs::read_to_string(tmp).map_err(|e| format!("cannot read {}: {e}", tmp.display()))
}

fn confirm(prompt: &str) -> Result<bool, String> {
    print!("{prompt}");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}
