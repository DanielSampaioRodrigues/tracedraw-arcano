# Browser version

The same editor compiled to WebAssembly (`wasm32-unknown-unknown`) and
drawn with WebGL 2 by eframe, as a demo to try TraceDraw without
installing it. CI publishes it to GitHub Pages; it is not one of the
release downloads. Each run keeps the folder as the `web-demo-site`
artifact (`tracedraw-<version>-web.zip`, a static folder: serve it from
any web server; opening `index.html` from disk does not work because
browsers refuse to load modules from `file://`).

## What differs from the desktop

| Feature | Desktop | Browser |
|---|---|---|
| Open, Import, scripts, palettes, CSV, ICC profiles, thesaurus | system file dialog | the browser's file picker; the file is read into memory under `/upload/<name>` and opened from there |
| Save, Save As | system save dialog (then the `.cdr` version, `files.md`) | a dialog with Format (TraceDraw or `.cdr`) and Version, then a download named after the document (`<title>`, or the name of the opened file, with the format's extension) |
| Export, Publish to PDF, Save as Template, palette and script saves | system save dialog | a download named after the document |
| Print | PDF opened in the system viewer | PDF downloaded; print it from the browser's viewer |
| Send To | PDF written to Desktop, Documents or a mail folder | PDF downloaded |
| Edit Bitmap (external editor) | system image editor | not available (a browser cannot hand files to programs) |
| Settings | `settings.json` in the configuration folder | browser local storage (key `tracedraw.settings`) |
| Recent documents | listed on the Welcome screen | not recorded (uploads have no path to reopen) |
| Fonts | installed system fonts | bundled at build time: Liberation Sans, Serif and Mono, DejaVu Sans, Noto Sans Devanagari and Bengali; the published page's font folder adds about a hundred free families (stand-ins of the same widths for common office faces, popular families, Chinese, Japanese, Korean, Arabic, Hebrew and Thai), each fetched the first time a drawing's text needs it (see `web/FONTS.md`); Chinese and Japanese UI text needs a font the build machine had |
| System clipboard | text and images (arboard) | text through the browser; pasting images from other programs is not available |
| Help links | system browser | a new tab |

## A demo, and it says so

The browser version is for trying TraceDraw without installing it, and
tells the visitor what that means and where the desktop app is:

- The loading page only says "Loading TraceDraw...". Each time the page
  opens, once TraceDraw is up, a message says it is a demo, slower on
  heavy work and limited to the fonts that come with it, with an OK
  button. The desktop downloads
  (`https://github.com/igtoth/tracedraw/releases/latest`) are linked
  from the Welcome Screen and the menu bar.
- The Welcome Screen's Get Started page shows the same in place of the
  recent documents (a browser keeps none), with a "Download the desktop
  app" button.
- The menu bar ends with a link to the downloads ("Browser demo: get the
  desktop app"); its tooltip repeats what the demo leaves out.

Heavy work is slower than on the desktop because the build is optimised
for download size (`profile.web`, `opt-level = "s"`), runs on one thread
(effects, tracing, mosaics and large exports keep the page busy until
they finish) and a page has less memory than a program.

## Keyboard

The editor's shortcuts reach the editor instead of the browser: while
the canvas has the focus, Ctrl (or Cmd) with a letter and the function
keys F1 to F12 are not passed to the browser, except Ctrl+C, Ctrl+X and
Ctrl+V (clipboard), Ctrl+R (reload) and Ctrl+Shift+I (developer tools).
Ctrl+N, Ctrl+T and Ctrl+W stay with the browser because no page can take
them; use the File menu for New and Close.

## Building

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # must match Cargo.lock
cargo build --target wasm32-unknown-unknown --profile web -p tracedraw
wasm-bindgen --target web --no-typescript --out-dir site \
    target/wasm32-unknown-unknown/web/tracedraw.wasm
cp web/index.html web/FONTS.md site/
cp assets/icon/tracedraw-256.png site/icon-256.png
python3 -m http.server -d site 8080   # then open http://localhost:8080
```

`TRACEDRAW_WEB_FONTS` (folders separated by `:`) points the build at the
fonts to bundle; without it the usual Linux font folders are searched.

The font folder is optional: `python3 web/font_folder.py web/fonts.txt
site/fonts` builds it (downloading the google/fonts files at the commit
`web/fonts.txt` pins, or copying them from a checkout with
`--google-fonts DIR`; the TeX Gyre and DejaVu files come from the
`fonts-texgyre` and `fonts-dejavu-*` packages). It writes the files, a
`licences/` folder and `fonts.json`, which maps each family to its files.
When the page starts it reads `fonts/fonts.json`; the families join the
font lists, and text that needs one (set in it, set in a family it
stands in for, or holding letters only it has) asks for it: the page
fetches its files, the text engine loads them and the page draws again.
Without the folder the page works with the bundled fonts.
`.cargo/config.toml` selects the browser's random number source for
`getrandom`. `scripts/web/build.sh` runs these steps.
