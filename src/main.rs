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

const USAGE: &str = "usage: bure [-t|--type f|d]... [DIR]";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    File,
    Dir,
}

#[derive(Debug, PartialEq)]
struct Args {
    dir: PathBuf,
    /// Empty means every kind.
    types: Vec<Kind>,
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args().skip(1))?;
    let dir = &args.dir;
    let (shown, hidden): (Vec<_>, Vec<_>) = list_entries(dir)?
        .into_iter()
        .partition(|(kind, _)| args.types.is_empty() || args.types.contains(kind));
    if shown.is_empty() {
        println!("No matching entries in {}", dir.display());
        return Ok(());
    }
    let old: Vec<String> = shown
        .into_iter()
        .map(|(kind, name)| if kind == Kind::Dir { name + "/" } else { name })
        .collect();
    let unlisted: Vec<String> = hidden.into_iter().map(|(_, name)| name).collect();

    let tmp = env::temp_dir().join(format!("bure-{}.txt", std::process::id()));
    let edited = edit_names(&tmp, &old);
    let _ = fs::remove_file(&tmp);
    let renames = plan::plan(&old, &edited?, &unlisted).map_err(|e| e.to_string())?;

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

    rename::apply(dir, &renames)?;
    println!("Renamed {} entries", renames.len());
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut dir = None;
    let mut types = Vec::new();
    while let Some(arg) = args.next() {
        let value = match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "-t" | "--type" => Some(
                args.next()
                    .ok_or_else(|| format!("{arg} needs a value (f or d)\n{USAGE}"))?,
            ),
            _ => arg.strip_prefix("--type=").map(str::to_string),
        };
        if let Some(value) = value {
            types.push(match value.as_str() {
                "f" | "file" => Kind::File,
                "d" | "dir" | "directory" => Kind::Dir,
                _ => return Err(format!("unknown type {value:?} (use f or d)\n{USAGE}")),
            });
        } else if arg.starts_with('-') && arg != "-" {
            return Err(format!("unknown option {arg}\n{USAGE}"));
        } else if dir.replace(PathBuf::from(&arg)).is_some() {
            return Err(format!("unexpected argument {arg}\n{USAGE}"));
        }
    }
    Ok(Args {
        dir: dir.unwrap_or_else(|| ".".into()),
        types,
    })
}

/// Lists entries, directories first, like `ls --group-directories-first`.
fn list_entries(dir: &Path) -> Result<Vec<(Kind, String)>, String> {
    let read = fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut entries = read
        .map(|entry| {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = if entry.path().is_dir() {
                Kind::Dir
            } else {
                Kind::File
            };
            let name = entry
                .file_name()
                .into_string()
                .map_err(|n| format!("non-UTF-8 file name: {}", n.to_string_lossy()))?;
            Ok((kind, name))
        })
        .collect::<Result<Vec<_>, String>>()?;
    entries.sort_by(|(a_kind, a), (b_kind, b)| {
        (*b_kind == Kind::Dir)
            .cmp(&(*a_kind == Kind::Dir))
            .then_with(|| a.to_lowercase().cmp(&b.to_lowercase()))
            .then_with(|| a.cmp(b))
    });
    Ok(entries)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn defaults() {
        assert_eq!(
            parse(&[]),
            Ok(Args {
                dir: ".".into(),
                types: vec![]
            })
        );
    }

    #[test]
    fn type_forms() {
        let expected = Args {
            dir: "x".into(),
            types: vec![Kind::File, Kind::Dir],
        };
        assert_eq!(parse(&["-t", "f", "--type=d", "x"]), Ok(expected));
        assert_eq!(
            parse(&["--type", "directory"]).unwrap().types,
            vec![Kind::Dir]
        );
    }

    #[test]
    fn errors() {
        assert!(parse(&["-t"]).is_err());
        assert!(parse(&["-t", "x"]).is_err());
        assert!(parse(&["--files"]).is_err());
        assert!(parse(&["a", "b"]).is_err());
    }
}
