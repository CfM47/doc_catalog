# doclib

Tag-indexed catalog for books and articles. Metadata lives in a local SQLite
database, the files themselves live on an rclone remote, and a local cache
holds whatever you have read recently.

## Setup

1. Install [rclone](https://rclone.org/install/) and configure a remote:

   ```sh
   rclone config          # create a remote, e.g. named "gdrive"
   ```

2. Build and point the config at that remote:

   ```sh
   cargo build --release
   ./target/release/doclib config     # prints the config path
   ```

   ```toml
   remote = "gdrive:doclib"
   cache_max_bytes = 21474836480      # 20 GiB; 0 disables pruning

   [openers]
   pdf = "okular"
   ```

   Anything without an explicit opener falls back to `$DOCLIB_OPENER`, then
   `xdg-open`.

## Use

```sh
doclib import ~/books                 # walk a directory, prompt per file
doclib import ~/books --auto          # accept extracted metadata, no prompts
doclib import paper.pdf --kind article

doclib search knuth
doclib open knuth                     # search, pick, launch the reader
doclib list --tag compilers
doclib tag "art of computer"
doclib tags

doclib sync                           # re-upload anything missing remotely
doclib cache status
doclib cache prune --max 5000000000
```

`open` with no match, or several, drops into an interactive picker.

## How it works

Import copies each file into the cache, hashes it with blake3, uploads it under
`<remote>/<xx>/<hash>.<ext>`, and records the metadata locally. The hash is the
dedupe and integrity key; the document's identity is a UUID, so re-hashing a
modified file never orphans its tags.

Metadata comes from the file first (EPUB metadata is reliable, PDF info
dictionaries usually are not), then from an ISBN lookup against OpenLibrary or
a DOI lookup against Crossref. Remote records win over embedded fields.

Search is SQLite FTS5 over title, authors, publisher and journal, kept in sync
by triggers.

`cache prune` evicts least-recently-opened files once the cache exceeds the
ceiling. Evicted files are pulled back from the remote on the next `open`.
