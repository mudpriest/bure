use std::collections::HashSet;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum PlanError {
    LineCount { expected: usize, found: usize },
    InvalidName { line: usize, name: String },
    Duplicate(String),
    Taken(String),
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::LineCount { expected, found } => write!(
                f,
                "expected {expected} lines but found {found} (lines must not be added or removed)"
            ),
            PlanError::InvalidName { line, name } => {
                write!(f, "line {line}: invalid name {name:?}")
            }
            PlanError::Duplicate(name) => write!(f, "duplicate target name {name:?}"),
            PlanError::Taken(name) => {
                write!(f, "{name:?} is taken by an entry filtered out by --type")
            }
        }
    }
}

/// Matches line N of `edited` to `old[N]` and returns the `(old, new)` pairs that changed.
/// A trailing `/` (the directory marker) is stripped from both sides.
/// `unlisted` are entries that stay in place, so no line may claim their names.
pub fn plan(
    old: &[String],
    edited: &str,
    unlisted: &[String],
) -> Result<Vec<(String, String)>, PlanError> {
    let mut new: Vec<&str> = edited.lines().map(strip_slash).collect();
    while new.last() == Some(&"") {
        new.pop();
    }
    if new.len() != old.len() {
        return Err(PlanError::LineCount {
            expected: old.len(),
            found: new.len(),
        });
    }

    let taken: HashSet<String> = unlisted.iter().map(|n| fold(n)).collect();
    let mut seen = HashSet::new();
    for (i, name) in new.iter().enumerate() {
        if !is_valid(name) {
            return Err(PlanError::InvalidName {
                line: i + 1,
                name: name.to_string(),
            });
        }
        if taken.contains(&fold(name)) {
            return Err(PlanError::Taken(name.to_string()));
        }
        if !seen.insert(fold(name)) {
            return Err(PlanError::Duplicate(name.to_string()));
        }
    }

    Ok(old
        .iter()
        .map(|o| strip_slash(o))
        .zip(new)
        .filter(|(o, n)| o != n)
        .map(|(o, n)| (o.to_string(), n.to_string()))
        .collect())
}

fn strip_slash(name: &str) -> &str {
    name.strip_suffix('/').unwrap_or(name)
}

fn is_valid(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\'])
}

/// Case-insensitive file systems would treat `a` and `A` as the same entry.
fn fold(name: &str) -> String {
    if cfg!(any(windows, target_os = "macos")) {
        name.to_lowercase()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn pair(a: &str, b: &str) -> (String, String) {
        (a.to_string(), b.to_string())
    }

    #[test]
    fn unchanged_yields_nothing() {
        assert_eq!(plan(&names(&["a", "b"]), "a\nb\n", &[]), Ok(vec![]));
    }

    #[test]
    fn simple_rename() {
        assert_eq!(
            plan(&names(&["a", "b"]), "a\nz\n", &[]),
            Ok(vec![pair("b", "z")])
        );
    }

    #[test]
    fn swap() {
        assert_eq!(
            plan(&names(&["a", "b"]), "b\na\n", &[]),
            Ok(vec![pair("a", "b"), pair("b", "a")])
        );
    }

    #[test]
    fn crlf_and_trailing_blank_lines() {
        assert_eq!(
            plan(&names(&["a", "b"]), "x\r\nb\r\n\r\n\n", &[]),
            Ok(vec![pair("a", "x")])
        );
    }

    #[test]
    fn line_count_mismatch() {
        assert_eq!(
            plan(&names(&["a", "b"]), "a\n", &[]),
            Err(PlanError::LineCount {
                expected: 2,
                found: 1
            })
        );
    }

    #[test]
    fn empty_name() {
        assert_eq!(
            plan(&names(&["a", "b", "c"]), "a\n\nc\n", &[]),
            Err(PlanError::InvalidName {
                line: 2,
                name: String::new()
            })
        );
    }

    #[test]
    fn path_separator() {
        assert!(matches!(
            plan(&names(&["a"]), "x/y\n", &[]),
            Err(PlanError::InvalidName { line: 1, .. })
        ));
        assert!(matches!(
            plan(&names(&["a"]), "x\\y\n", &[]),
            Err(PlanError::InvalidName { line: 1, .. })
        ));
    }

    #[test]
    fn duplicate_targets() {
        assert_eq!(
            plan(&names(&["a", "b", "c"]), "x\nx\nc\n", &[]),
            Err(PlanError::Duplicate("x".into()))
        );
    }

    #[test]
    fn collision_with_unchanged_entry() {
        assert_eq!(
            plan(&names(&["a", "b"]), "b\nb\n", &[]),
            Err(PlanError::Duplicate("b".into()))
        );
    }

    #[test]
    fn directory_marker_is_stripped() {
        assert_eq!(
            plan(&names(&["d/", "e/", "f"]), "x/\ne\nf\n", &[]),
            Ok(vec![pair("d", "x")])
        );
    }

    #[test]
    fn collision_with_unlisted_entry() {
        assert_eq!(
            plan(
                &names(&["a", "b"]),
                "a
d
",
                &names(&["d"])
            ),
            Err(PlanError::Taken("d".into()))
        );
    }

    #[test]
    fn case_only_rename_is_allowed() {
        assert_eq!(plan(&names(&["a"]), "A\n", &[]), Ok(vec![pair("a", "A")]));
    }
}
