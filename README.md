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
[![rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)

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
- **Stored in plain folders.** Your disk, a mounted USB stick, an external
  drive. No daemon, no account, no other tool to install.
- **Replicated across as many folders as you like**, converging with one
  command. Delete a book with the USB disk unplugged and it stays deleted.
- **Content-addressed.** Files are keyed by BLAKE3 hash, so duplicates are
  caught on import and renames are free.
- **Local cache with LRU eviction** — read a book once, keep it until space runs
  short, copy it back on demand.
- **Export to an e-reader** under a name a human can read, rather than a hash.
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

Requires Rust 1.88+ (2024 edition). Nothing else.

## Quick start

```sh
# 1. say where the library lives (defaults to ~/doclib)
doclib config --store ~/doclib /mnt/usb/library

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
| `doclib import <path>` | Catalog files into every store. `--auto` skips prompts, `--kind` forces book/article |
| `doclib search [query]` | Full-text search, printed as a table |
| `doclib open [query]` | Search, pick, launch your reader |
| `doclib copy [query] --to <dir>` | Export under a readable name. `--tag` copies a whole shelf |
| `doclib show [query]` | Every stored field for one document |
| `doclib edit [query]` | Fix metadata. `--lookup` re-fetches from OpenLibrary/Crossref |
| `doclib delete [query]` | Remove from the catalog. `--purge` also deletes the file |
| `doclib list` | List everything. `--tag`, `--kind` to filter |
| `doclib tag [query]` | Edit one document's tags |
| `doclib tags` | Every tag with its document count |
| `doclib stats` | Totals, cache usage, and where metadata is missing |
| `doclib export [file]` | Write the catalog to JSON (stdout if no file) |
| `doclib restore <file>` | Merge a JSON backup back in |
| `doclib sync` | Verify every store holds every catalogued document |
| `doclib update` | Copy files between stores until they match. `--dry-run` to look first |
| `doclib purge` | Delete stored files no catalog entry points at |
| `doclib destroy` | Delete the whole library, behind a typed confirmation |
| `doclib cache status\|prune` | Inspect or shrink the local cache |
| `doclib config` | Open the config in `$EDITOR`. `--store`, `--show`, `--reset` |

Commands taking `[query]` fall through to an interactive picker when the query
matches nothing or matches several things — so `doclib open` on its own is a
perfectly good way to browse.

## Configuration

`doclib config` opens the file in `$VISUAL`, then `$EDITOR`, then `vi`.
`doclib config --reset` starts over, keeping the old file as `.bak`.

```toml
stores = ["/home/you/doclib", "/mnt/usb/library"]
cache_max_bytes = 21474836480   # 20 GiB; 0 to never evict

[openers]
pdf = "okular"
epub = "foliate"
```

A store is an ordinary directory — on this disk, on a mounted USB stick, on an
external drive, on a network share the system has already mounted. Paths must
be absolute; `~` is expanded.

Every store carries a `.doclib-store` marker. That is how an unmounted disk is
told apart from an empty folder: without the marker doclib refuses to write
there, so a disconnected USB stick is never mistaken for an empty library.

## Several stores

Documents are written to every store on import. When one was disconnected, or
you imported from another machine, `update` makes them match again:

```sh
doclib update --dry-run   # what would be copied where
doclib update
```

Stores converge on the *union* of what they hold, so a file present in one
appears in all of them. Deletions are recorded, so a book removed while the USB
disk was unplugged is not resurrected when you plug it back in — `doclib purge`
finishes the job on that disk once it returns.

## Copying to an e-reader

The stores are content-addressed, which is right for a library and useless on a
device: `92/920c6224….pdf` tells you nothing in a file list.

```sh
doclib copy knuth --to /run/media/you/KOBO/books
doclib copy --tag number-theory --to /run/media/you/KOBO/books
```

Writes `Knuth - The Art of Computer Programming (1997).pdf`, sanitised for FAT.
Re-running skips files already there rather than duplicating them.

## Backing up

Your files live in the stores and can be copied back. Your tags and corrections
live in exactly one SQLite file.

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

[![Star this repo](https://shields.io)](https://github.com)

## License

MIT — see [LICENSE](LICENSE).
