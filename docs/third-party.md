# Third-party code

TraceDraw takes code from other open-source projects only with the project
owner's approval, and only under a licence that allows it (MIT,
Apache-2.0 or their dual form like our own `MIT OR Apache-2.0`). This page
records what came from where, at which commit, and what the licence asks
of us. The licence texts and notices stay verbatim in `licenses/`.

## Rules

1. A piece of code is taken whole, at a pinned commit, never updated
   behind our back: the commit is written here and in `ATTRIBUTION.md`.
2. Its copyright notice, its licence texts and its `NOTICE` file are
   copied into `licenses/<project>/`.
3. We say what we changed, in `ATTRIBUTION.md`, when we adapt a file.
4. Its name appears only in the copied files' headers, in `licenses/`,
   `ATTRIBUTION.md` and this page. It never appears in the interface,
   the prose, the menu, the file names or the commit messages (see rule 3
   of `AGENTS.md`).
5. The project's name, wordmark and logos are trademarks and are not
   covered by the licence: we never use them.
6. Third-party material a notice carries (fonts, hyphenation patterns,
   colour profiles, sample files) keeps its own licence, listed in that
   notice; it is not relicensed by ours.

## Sources and pins

The Creative Apps suite of the same publisher (page layout, vector,
photo, spreadsheets, slides) is `MIT OR Apache-2.0`, written in Rust:

| Source | Pinned commit | Taken so far | Licence texts |
|---|---|---|---|
| DesignCraft (`storytold/designcraft`) | `d848948b112d49af1812e5941b557e19b5a61b91` | nothing yet | `licenses/designcraft/` |
| VectorCraft (`storytold/vectorcraft`) | `0a702cc0a5f36346b420b7effc5698ef36b05b2d` | nothing yet | `licenses/vectorcraft/` |
| PhotoCraft (`storytold/photocraft`) | `aae9b7ea61195eba0c43877d59654aefdc7c5576` | nothing yet | `licenses/photocraft/` |
| craft-libs (`storytold/craft-libs`) | `058b047760dfe5ba51f6f526faa3b0c833552dfa` | nothing yet | `licenses/craft-libs/` |
| indd-utils (`storytold/indd-utils`) | `136f973a9d5f1b3ef34e46e27f4a08af196efde7` | nothing yet | `licenses/indd-utils/` |

## Planned, in the order the stages come

- `.indd` import through the indd-utils reader, which turns a `.indd`
  into IDML; IDML read by our own reader (`crates/io`), written from the
  published format notes and from files we produce.
- IDML save, the same way.
- Shape operations (divide, outline, merge) from the vector app's
  `pathops` crate, a pure library with no interface.
- Vector tools that are missing: transform each, polar grid, path eraser.
- Photo tools from the photo app's `ops` and `cms` crates.
