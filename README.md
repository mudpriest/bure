# bure

Bulk-rename the entries of a directory in your text editor.

## Install

```sh
cargo install --path .
```

This builds a release binary and copies it to `~/.cargo/bin` (`%USERPROFILE%\.cargo\bin` on Windows). rustup puts that directory on your PATH, so `bure` works from anywhere in a new terminal.

- Update: run the same command again after changing the code.
- Uninstall: `cargo uninstall bure`

## Usage

```sh
bure [DIR]   # DIR defaults to the current directory
```

1. bure writes the entry names of `DIR` to a temp file, one per line: directories first (marked with a trailing `/`), then files, each group sorted by name ignoring case.
2. It opens the file in `$VISUAL`, `$EDITOR`, or a default (`notepad` on Windows, `vi` elsewhere).
3. Edit the names, save, and close the editor.
4. bure lists the planned renames and asks for confirmation before applying them.

## Rules

- Line N renames entry N. Don't add, remove, or reorder lines.
- Names must not be empty, `.`, `..`, or contain `/` or `\`. The trailing `/` on directories is only a marker; keeping or removing it makes no difference.
- Target names must be unique. The check ignores case on Windows and macOS.
- Swaps and cycles (`a -> b`, `b -> a`) work, because each entry is first moved to a temporary name.

## Development

```sh
cargo test
```
