<div align="center">

```
    ██████╗  ██████╗  ██████╗██╗     ██╗██████╗     ( (
    ██╔══██╗██╔═══██╗██╔════╝██║     ██║██╔══██╗      ) )
    ██║  ██║██║   ██║██║     ██║     ██║██████╔╝   ********
    ██║  ██║██║   ██║██║     ██║     ██║██╔══██╗   ████████▀▌
    ██████╔╝╚██████╔╝╚██████╗███████╗██║██████╔╝   ▀██████▀▀
    ╚═════╝  ╚═════╝  ╚═════╝╚══════╝╚═╝╚═════╝     ▀▀▀▀▀▀
```

**A blazingly-fast ⚡ memory-safe 💾 tag-indexed shelf for books 📚 and articles 📄**

[![crates.io](https://img.shields.io/crates/v/doclib.svg)](https://crates.io/crates/doclib)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

</div>

---

Folders are a bad way to organise a library. A book about compiler design is
also a book about automata, also a reference you keep coming back to, also a
PDF you have not opened since 2019 — and a directory tree makes you pick one.

`doclib` keeps the files on cloud storage and the *meaning* in a local SQLite
database. Tag freely, search by anything, open a book from the terminal in one
command.

```console
$ doclib open dragon
opened "Compilers Principles Techniques and Tools" with okular

$ doclib list --tag automata
TITLE                                         AUTHORS                     KIND     YEAR
Regulated grammars and automata               Alexander Meduna, Petr Ze…  book     2014
```

## Features

- **Tags, not folders.** A document belongs to as many tags as you like.
- **Full-text search** over titles, authors, publishers and journals, via SQLite FTS5.
- **Books and articles** in one catalog — ISBNs resolve against OpenLibrary,
  DOIs against Crossref, so one identifier fills in the whole record.
- **Any storage rclone speaks:** Google Drive, Dropbox, S3, Backblaze, a USB disk.
- **Content-addressed.** Files are keyed by BLAKE3 hash, so duplicates are
  caught on import and renames are free.
- **Local cache with LRU eviction** — read a book once, keep it until space runs
  short, re-fetch on demand.
- **JSON export.** The files are replaceable; your tags are not. Back them up
  in a format `git diff` can read.
- **Conversational prompts** in the style of `gh`, not a wall of flags.

## Install

```sh
cargo install doclib
```

From source:

```sh
cargo install --git https://github.com/CfM47/doc_catalog
```

### Requirements

- Rust 1.85+ (2024 edition)
- [rclone](https://rclone.org/install/) on `PATH`, with a remote configured

```sh
rclone config          # create a remote
rclone listremotes     # confirm the name
```

## Quick start

```sh
# 1. point doclib at your storage
doclib config --remote "gdrive:doclib"

# 2. catalog a pile of files
doclib import ~/unsorted-books

# 3. use it
doclib open knuth
doclib list --tag algorithms
doclib stats
```

`import` walks a directory, hashes each file, pulls what metadata it can from
the file itself, offers a lookup by ISBN or DOI, then asks you to confirm. In a
hurry, `--auto` accepts everything unattended and you can correct later with
`doclib edit`.

## Commands

| Command | What it does |
| --- | --- |
| `doclib import <path>` | Catalog and upload files. `--auto` skips prompts, `--kind` forces book/article |
| `doclib search [query]` | Full-text search, printed as a table |
| `doclib open [query]` | Search, pick, launch your reader |
| `doclib show [query]` | Every stored field for one document |
| `doclib edit [query]` | Fix metadata. `--lookup` re-fetches from OpenLibrary/Crossref |
| `doclib delete [query]` | Remove from the catalog. `--purge` also deletes the stored file |
| `doclib list` | List everything. `--tag`, `--kind` to filter |
| `doclib tag [query]` | Edit one document's tags |
| `doclib tags` | Every tag with its document count |
| `doclib stats` | Totals, cache usage, and where metadata is missing |
| `doclib export [file]` | Write the catalog to JSON (stdout if no file) |
| `doclib restore <file>` | Merge a JSON backup back in |
| `doclib sync` | Verify the remote holds every catalogued document |
| `doclib purge` | Delete stored files no catalog entry points at |
| `doclib cache status\|prune` | Inspect or shrink the local cache |
| `doclib config` | Open the config in `$EDITOR`, or `--remote` to set one value |

Commands taking `[query]` fall through to an interactive picker when the query
matches nothing or matches several things — so `doclib open` on its own is a
perfectly good way to browse.

## Configuration

`doclib config` opens the file in `$VISUAL`, then `$EDITOR`, then `vi`.

```toml
remote = "gdrive:doclib"
cache_max_bytes = 21474836480   # 20 GiB; 0 to never evict

[openers]
pdf = "okular"
epub = "foliate"
```

`remote` is passed to rclone verbatim, so every form it understands works:

| Value | Meaning |
| --- | --- |
| `gdrive:doclib` | a configured remote, in a subfolder |
| `b2:my-bucket/doclib` | bucket storage with a path |
| `:s3:my-bucket` | an inline connection string |
| `/mnt/usb/doclib` | a plain absolute path, no remote needed |

## Backing up

Your files live on the remote and can be re-downloaded. Your tags and
corrections live in exactly one SQLite file.

```sh
doclib export ~/library/catalog.json
git -C ~/library commit -am "catalog $(date -I)"
```

Restoring merges rather than overwrites — documents already present are left
alone, so it is safe to run against a live catalog.

```sh
doclib restore catalog.json
doclib sync
```

## License

MIT — see [LICENSE](LICENSE).
