# git-author-reformer

A terminal UI tool for rewriting git commit author history.

Rename primary commit authors, fix emails, and drop `Co-authored-by` trailers across an entire repository in seconds. Ships as a single pre-built binary: download with one command, then run directly.

https://github.com/user-attachments/assets/ebc8b9f3-3d03-49bb-ae43-f58785269ead

## Quick Start

Run directly without installing:

```sh
curl -fsSL https://raw.githubusercontent.com/uinstinct/git-author-reformer/main/install.sh | sh
```

The script detects your platform, downloads the matching binary, verifies its SHA256 checksum, and saves it as `./git-author-reformer` in the current directory. Re-running the script reuses the existing binary — no re-download needed. Then run it directly:

```sh
./git-author-reformer
```

## What It Does

- **Rename authors** — change name and/or email on commits attributed to a given identity
- **Bulk rewrite** — rewrites every commit reachable from every branch and tag
- **Drop co-authors** — remove specific `Co-authored-by` trailer lines from commit messages

## Usage

Run the tool from inside a git repository:

```sh
cd /path/to/your/repo
curl -fsSL https://raw.githubusercontent.com/uinstinct/git-author-reformer/main/install.sh | sh
./git-author-reformer
```

Use the keyboard to:

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate the author list |
| `Enter` | Select an author to rename |
| `Tab` | Switch between fields |
| `Ctrl+S` | Confirm and rewrite history |
| `q` / `Esc` | Quit without changes |

> **Note:** Run this on a local clone. Rewriting history changes commit SHAs — you will need to force-push if the repository has a remote.

## Platform Support

Pre-built binaries are available for:

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `git-author-reformer-linux-x86_64` (static musl) |
| macOS Apple Silicon | `git-author-reformer-macos-aarch64` |
| macOS Intel | `git-author-reformer-macos-x86_64` |

The Linux binary is fully static (musl) — no glibc dependency, runs on any Linux kernel ≥ 3.2.

## Building from Source

Requires Rust 1.74+.

```sh
git clone https://github.com/uinstinct/git-author-reformer.git
cd git-author-reformer
cargo build --release
./target/release/git-author-reformer
```

For the fully static Linux binary:

```sh
rustup target add x86_64-unknown-linux-musl
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release --target x86_64-unknown-linux-musl
```

## License

MIT
