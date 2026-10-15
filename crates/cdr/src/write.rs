//! `.cdr` writer for every version from 7 to 22.
//!
//! The chunk tree follows the layout of real files of each version, record
//! by record (`docs/cdr-format.md`, Writing, lists each record and its
//! sources): versions 7 to 13 are a RIFF file; versions 14 and 15
//! keep that RIFF stream in a ZIP archive as `content/riffData.cdr`; from
//! version 16 the RIFF stream (`content/root.dat`) holds the chunk tree and
//! each payload lives in a data file of the archive (`content/data/*.dat`),
//! reached through a 16-byte redirect record. Coordinates are 1/254000 inch
//! with the origin at the page centre.
//!
//! What is written: pages (after a master page with the guide, desktop and
//! grid layers files carry), layers with names, rectangles, ellipses (with
//! arcs), curves for every other outline (polygons, paths, tables, symbols,
//! expanded effects), groups, artistic text with one font, size and style
//! per run, bitmaps (24-bit), solid and fountain fills, outlines (width,
//! caps, joins, dashes, nib, "scale with object", "behind fill") and
//! uniform transparency. A clip frame holding one bitmap becomes that
//! bitmap with the frame as its crop path; other clip frames a group of
//! the frame outline and the contents. Rectangles of versions 15 and later
//! are written as the curve they draw (their corner records are not
//! understood well enough to write).

use crate::parse::{
    ARG_COORDS, ARG_FILL, ARG_NAME, ARG_OPACITY, ARG_OUTLINE, ARG_STYLE, ARG_STYLE_STRING,
    OBJ_ARTISTIC_TEXT, OBJ_BITMAP, OBJ_CURVE, OBJ_ELLIPSE, OBJ_RECT,
};
use std::collections::HashMap;
use std::io::Write as _;
use tracedraw_core::{
    document::{Shape, ShapeKind, Symbol, TextSpan},
    geometry::{Affine, BezPath, PathEl, Point, Rect, Shape as _},
    style::{FountainKind, LineCap, LineJoin},
    Color, Document, Fill, Stroke, TextAlign,
};

/// The oldest version the writer produces.
pub const OLDEST_VERSION: u16 = 7;
/// The newest version the writer produces.
pub const NEWEST_VERSION: u16 = 22;
/// The version [`document_to_cdr`] writes.
pub const DEFAULT_VERSION: u16 = 12;

/// The versions [`document_to_cdr_version`] can write, oldest first.
pub fn versions() -> std::ops::RangeInclusive<u16> {
    OLDEST_VERSION..=NEWEST_VERSION
}

/// Write the whole document as a version 12 `.cdr` file.
pub fn document_to_cdr(doc: &Document) -> Vec<u8> {
    document_to_cdr_version(doc, DEFAULT_VERSION)
}

/// Write the whole document as a `.cdr` file of `version` (7 to 22; other
/// values are brought into that range).
pub fn document_to_cdr_version(doc: &Document, version: u16) -> Vec<u8> {
    let v = Ver(version.clamp(OLDEST_VERSION, NEWEST_VERSION));
    let mut w = Writer::new(v, &doc.symbols, &doc.title);
    let tree = w.document(doc);
    if v.redirects() {
        let (root, files) = pack_redirected(v, &tree);
        zip_v16(&root, &files)
    } else {
        let riff = riff_file(v.form(), &tree);
        if v.zip() {
            zip_v14(&riff)
        } else {
            riff
        }
    }
}

/// Coordinate units per millimetre.
const UNITS_PER_MM: f64 = 254000.0 / 25.4;
/// The "since version" tag records of version 13 and later start with.
const SINCE_13: u32 = 1300;
/// Halftone screen of fills and outlines: 45 degrees, 60 lines per inch.
const SCREEN: [u8; 8] = [0x40, 0xa5, 0xae, 0x02, 0x3c, 0x00, 0x00, 0x00];
/// A full ellipse is stored from 270 to 270 degrees (millionths).
const FULL_ELLIPSE_ANGLE: i32 = 270_000_000;
/// Argument types this writer adds to those `parse.rs` names.
const ARG_ID: u32 = 100;
const ARG_LAYER_COLOR: u32 = 2000;
const ARG_PROPS: u32 = 12000;
const ARG_GUID: u32 = 40050;
/// On text objects of version 16 and later: 1.
const ARG_TEXT_LINK: u32 = 16015;
/// On guides layers of version 17 and later: 2.
const ARG_GUIDES_MODE: u32 = 16001;
/// Layer kinds stored as the `loda` object type of a layer.
const LAYER_PLAIN: u32 = 0;
const LAYER_GRID: u32 = 0x0b;
const LAYER_GUIDES: u32 = 0x0c;
const LAYER_DESKTOP: u32 = 0x11;
/// The first text style; text paragraphs refer to the one of their
/// alignment (`style_for`).
const STYLE_ID: u32 = 1;
/// The style graphics refer to (before version 16, argument 200).
const GRAPHIC_STYLE_ID: u32 = STYLE_ID + STYLE_ALIGNS.len() as u32;

/// Alignments of the style table, in style id order from `STYLE_ID`:
/// left, centre, right, justify, as alignment codes (0 none, 1 left,
/// 2 centre, 3 right, 4 full justify, 5 forced; the style strings'
/// numbering too).
const STYLE_ALIGNS: [(TextAlign, u32); 4] = [
    (TextAlign::Left, 1),
    (TextAlign::Center, 2),
    (TextAlign::Right, 3),
    (TextAlign::Justify, 4),
];

/// The style of text with this alignment.
fn style_for(align: TextAlign) -> u32 {
    STYLE_ALIGNS
        .iter()
        .position(|(a, _)| *a == align)
        .map_or(STYLE_ID, |i| STYLE_ID + i as u32)
}

/// The version being written and what it implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ver(u16);

impl Ver {
    /// `CDR` and the version letter: digits up to 9, then `A` (10) to `H`
    /// (17) and `J` (18) on; `I` is not used.
    fn form(self) -> [u8; 4] {
        let c = match self.0 {
            v @ 0..=9 => b'0' + v as u8,
            v @ 10..=17 => b'A' + (v - 10) as u8,
            v => b'J' + (v.min(34) - 18) as u8,
        };
        [b'C', b'D', b'R', c]
    }
    /// Tagged fill, outline, transform and configuration records.
    fn tagged(self) -> bool {
        self.0 >= 13
    }
    /// A ZIP archive around the RIFF stream.
    fn zip(self) -> bool {
        self.0 >= 14
    }
    /// Payloads in data files, reached through redirect records.
    fn redirects(self) -> bool {
        self.0 >= 16
    }
    /// Names and text in UTF-16 (8-bit before version 12).
    fn unicode(self) -> bool {
        self.0 >= 12
    }
    /// Objects carry their style as a string (argument 201).
    fn style_strings(self) -> bool {
        self.0 >= 16
    }
    /// Style strings and text one byte per character (UTF-16 before).
    fn byte_strings(self) -> bool {
        self.0 >= 17
    }
    /// Rectangles keep their own record (from 15 on their layout changes).
    fn native_rects(self) -> bool {
        self.0 < 15
    }
}

// ---- bytes -------------------------------------------------------------

#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn u8(&mut self, x: u8) -> &mut Self {
        self.0.push(x);
        self
    }
    fn u16(&mut self, x: u16) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn u32(&mut self, x: u32) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn i32(&mut self, x: i32) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn f64(&mut self, x: f64) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.0.extend_from_slice(b);
        self
    }
    fn zeros(&mut self, n: usize) -> &mut Self {
        self.0.resize(self.0.len() + n, 0);
        self
    }
    /// Zeros up to `len` bytes in all (nothing when already longer).
    fn pad_to(&mut self, len: usize) -> &mut Self {
        if self.0.len() < len {
            self.0.resize(len, 0);
        }
        self
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn done(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

fn units(mm: f64) -> i32 {
    if !mm.is_finite() {
        return 0;
    }
    (mm * UNITS_PER_MM)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

fn utf16z(s: &str) -> Vec<u8> {
    let mut v = utf16(s);
    v.extend_from_slice(&[0, 0]);
    v
}

/// Windows-1252 bytes of `s`, or `None` when a character has none.
fn cp1252(s: &str) -> Option<Vec<u8>> {
    const HIGH: [u16; 32] = [
        0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
        0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc,
        0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
    ];
    s.chars()
        .map(|c| {
            let u = c as u32;
            if u < 0x80 || (0xa0..0x100).contains(&u) {
                Some(u as u8)
            } else {
                HIGH.iter()
                    .position(|h| *h != 0 && u32::from(*h) == u)
                    .map(|i| 0x80 + i as u8)
            }
        })
        .collect()
}

/// A layer or object name: UTF-16 from version 12, 8-bit before.
fn name_bytes(v: Ver, s: &str) -> Vec<u8> {
    if v.unicode() {
        utf16z(s)
    } else {
        let mut b = cp1252(s).unwrap_or_else(|| s.bytes().filter(|c| c.is_ascii()).collect());
        b.push(0);
        b
    }
}

/// A style string argument or record: its length and text (UTF-16 before
/// version 17, counted in units; bytes after).
fn style_string(v: Ver, s: &str) -> Vec<u8> {
    let mut b = Buf::default();
    if v.byte_strings() {
        b.u32(s.len() as u32).bytes(s.as_bytes());
    } else {
        let units: Vec<u8> = utf16(s);
        b.u32((units.len() / 2) as u32).bytes(&units);
    }
    b.done()
}

/// A random-looking GUID (version 4, as files carry them, in the byte
/// order files store), the same for the same seed and counter.
fn guid(seed: u64, n: u64) -> [u8; 16] {
    let mut x = seed ^ n.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut out = [0u8; 16];
    for chunk in out.chunks_mut(8) {
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        x ^= x >> 33;
        chunk.copy_from_slice(&x.to_le_bytes());
    }
    // Version 4 in the high bits of the third field, variant 10 in the
    // fourth.
    out[7] = (out[7] & 0x0f) | 0x40;
    out[8] = (out[8] & 0x3f) | 0x80;
    out
}

// ---- chunk tree --------------------------------------------------------

#[derive(Debug, Clone)]
enum Node {
    Chunk([u8; 4], Vec<u8>),
    List([u8; 4], Vec<Node>),
    /// A list whose body after its type is a record, not chunks (the
    /// style table before version 16).
    RawList([u8; 4], Vec<u8>),
}

fn chunk(id: &[u8; 4], data: Vec<u8>) -> Node {
    Node::Chunk(*id, data)
}

fn list(kind: &[u8; 4], children: Vec<Node>) -> Node {
    Node::List(*kind, children)
}

fn put_chunk(out: &mut Vec<u8>, id: &[u8], payload: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

fn serialize(nodes: &[Node], out: &mut Vec<u8>) {
    for n in nodes {
        match n {
            Node::Chunk(id, data) => put_chunk(out, id, data),
            Node::List(kind, children) => {
                let mut body = kind.to_vec();
                serialize(children, &mut body);
                put_chunk(out, b"LIST", &body);
            }
            Node::RawList(kind, data) => {
                let mut body = kind.to_vec();
                body.extend_from_slice(data);
                put_chunk(out, b"LIST", &body);
            }
        }
    }
}

/// A RIFF file of the form type and chunks.
fn riff_file(form: [u8; 4], nodes: &[Node]) -> Vec<u8> {
    let mut body = form.to_vec();
    serialize(nodes, &mut body);
    let mut out = Vec::with_capacity(body.len() + 8);
    put_chunk(&mut out, b"RIFF", &body);
    out
}

/// The data files of a version 16+ archive, in the order they are listed.
#[derive(Default)]
struct DataFiles {
    names: Vec<String>,
    data: Vec<Vec<u8>>,
}

impl DataFiles {
    fn index(&mut self, name: &str) -> usize {
        match self.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                self.names.push(name.to_string());
                self.data.push(Vec::new());
                self.names.len() - 1
            }
        }
    }
}

/// Version 16+: the RIFF stream with every chunk a 16-byte record. Up to
/// 8 bytes stay in the record (stream 0xFFFFFFFF); larger payloads go to
/// the data file of their top-level list (the bitmaps, the document
/// tables, the master page, each page).
fn pack_redirected(v: Ver, tree: &[Node]) -> (Vec<u8>, DataFiles) {
    let mut files = DataFiles::default();
    let mut body = v.form().to_vec();
    let mut page = 0usize;
    for node in tree {
        let file = match node {
            Node::List(kind, _) => match kind {
                b"bmpt" => Some("Bitmaps.dat".to_string()),
                b"doc " => Some("data1.dat".to_string()),
                b"page" => {
                    page += 1;
                    Some(if page == 1 {
                        "masterPage.dat".to_string()
                    } else {
                        format!("page{}.dat", page - 1)
                    })
                }
                _ => None,
            },
            _ => None,
        };
        let index = file.map(|f| files.index(&f));
        redirect(std::slice::from_ref(node), index, &mut files, &mut body);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    put_chunk(&mut out, b"RIFF", &body);
    (out, files)
}

fn redirect(nodes: &[Node], file: Option<usize>, files: &mut DataFiles, out: &mut Vec<u8>) {
    for n in nodes {
        match n {
            Node::List(kind, children) => {
                let mut body = kind.to_vec();
                redirect(children, file, files, &mut body);
                put_chunk(out, b"LIST", &body);
            }
            Node::RawList(kind, data) => {
                let mut body = kind.to_vec();
                body.extend_from_slice(data);
                put_chunk(out, b"LIST", &body);
            }
            Node::Chunk(id, data) => {
                let mut rec = Buf::default();
                match file.and_then(|f| files.data.get_mut(f).map(|d| (f, d))) {
                    Some((f, d)) if data.len() > 8 => {
                        let ofs = d.len();
                        d.extend_from_slice(data);
                        rec.u32(f as u32)
                            .u32(data.len() as u32)
                            .u32(ofs as u32)
                            .u32(0);
                    }
                    _ => {
                        let mut inline = data.clone();
                        inline.resize(8, 0);
                        rec.u32(u32::MAX)
                            .u32(data.len().min(8) as u32)
                            .bytes(&inline[..8]);
                    }
                }
                put_chunk(out, id, &rec.done());
            }
        }
    }
}

fn zip_v14(riff: &[u8]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let ok = zip.start_file("content/riffData.cdr", stored).is_ok() && zip.write_all(riff).is_ok();
    match zip.finish() {
        Ok(c) if ok => c.into_inner(),
        _ => Vec::new(),
    }
}

fn zip_v16(root: &[u8], files: &DataFiles) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut ok = true;
    for (name, data) in files.names.iter().zip(&files.data) {
        ok &= zip
            .start_file(format!("content/data/{name}"), deflated)
            .is_ok()
            && zip.write_all(data).is_ok();
    }
    ok &= zip.start_file("content/dataFileList.dat", deflated).is_ok()
        && zip.write_all(files.names.join("\n").as_bytes()).is_ok();
    ok &= zip.start_file("content/root.dat", deflated).is_ok() && zip.write_all(root).is_ok();
    match zip.finish() {
        Ok(c) if ok => c.into_inner(),
        _ => Vec::new(),
    }
}

// ---- records -----------------------------------------------------------

/// 12-byte colour record: model, palette, 4 unknown bytes, 4 values. CMYK
/// keeps its percentages (model 2), grey its level (model 9); everything
/// else is written as sRGB (model 5, stored B G R). A spot colour is
/// written as its process stand-in.
fn colour(c: Color) -> [u8; 12] {
    let (model, values) = match c.process() {
        Color::Cmyk { c, m, y, k } => {
            let pct = |x: f32| (x.clamp(0.0, 1.0) * 100.0).round() as u8;
            (2u16, [pct(c), pct(m), pct(y), pct(k)])
        }
        Color::Gray { v } => (9, [(v.clamp(0.0, 1.0) * 255.0).round() as u8, 0, 0, 0]),
        Color::Registration => (2, [100, 100, 100, 100]),
        other => {
            let [r, g, b] = other.to_rgb8();
            (5, [b, g, r, 0])
        }
    };
    let mut out = [0u8; 12];
    out[0..2].copy_from_slice(&model.to_le_bytes());
    out[2..4].copy_from_slice(&5u16.to_le_bytes());
    out[8..12].copy_from_slice(&values);
    out
}

/// Version 13+: a colour as a property list (type 1, then from version 15
/// an empty palette entry id, type 7), closed by an empty record.
fn colour_props(v: Ver, c: Color) -> Vec<u8> {
    let mut b = Buf::default();
    b.u8(1).u32(12).bytes(&colour(c));
    if v.0 >= 15 {
        b.u8(7).u32(16).zeros(16);
    }
    b.u8(0).u32(0);
    b.done()
}

/// A colour as style strings write it.
fn colour_spec(c: Color) -> String {
    const NO_ID: &str = "00000000-0000-0000-0000-000000000000";
    match c {
        Color::Cmyk { c, m, y, k } => {
            let pct = |x: f32| (x.clamp(0.0, 1.0) * 100.0).round() as u8;
            format!(
                "CMYK,USER,{},{},{},{},100,{NO_ID}",
                pct(c),
                pct(m),
                pct(y),
                pct(k)
            )
        }
        other => {
            let [r, g, b] = other.to_rgb8();
            format!("RGB255,USER,{r},{g},{b},100,{NO_ID}")
        }
    }
}

/// The colour a fill shows as one colour (gradients: their first stop).
fn fill_colour(fill: &Fill) -> Option<Color> {
    match fill {
        Fill::None => None,
        Fill::Solid(c) => Some(*c),
        Fill::Fountain(f) => f.stops.first().map(|s| s.color),
        other => other.preview_color(),
    }
}

fn fountain_type(kind: FountainKind) -> u8 {
    match kind {
        FountainKind::Linear => 1,
        FountainKind::Radial => 2,
        FountainKind::Conical => 3,
        FountainKind::Square => 4,
    }
}

/// `fild`: a fill record. See `docs/cdr-format.md` for both layouts.
fn fild(v: Ver, id: u32, fill: &Fill) -> Vec<u8> {
    let fill = match fill {
        Fill::None | Fill::Solid(_) | Fill::Fountain(_) => fill.clone(),
        other => other.preview_color().map(Fill::Solid).unwrap_or(Fill::None),
    };
    let mut b = Buf::default();
    if !v.tagged() {
        b.u32(id);
        match &fill {
            Fill::Solid(c) => {
                b.u16(1).zeros(2).bytes(&colour(*c)).zeros(8).bytes(&SCREEN);
            }
            Fill::Fountain(f) => {
                b.u16(2).zeros(2).u8(fountain_type(f.kind));
                b.zeros(11).bytes(&SCREEN);
                b.i32((f.edge_pad * 100.0).round() as i32)
                    .i32((f.angle * 1_000_000.0).round() as i32)
                    .i32((f.offset.x * 100.0).round() as i32)
                    .i32((f.offset.y * 100.0).round() as i32)
                    .zeros(2)
                    .bytes(&[3, 0, 1, 0])
                    .u8(50)
                    .u8(0);
                let stops: Vec<_> = f.stops.iter().take(64).collect();
                b.u32(stops.len() as u32);
                for s in stops {
                    b.bytes(&colour(s.color))
                        .u32((s.pos.clamp(0.0, 1.0) * 100.0).round() as u32);
                }
            }
            _ => {
                b.u16(0).zeros(2);
            }
        }
        return b.done();
    }
    // Version 13+: id, tag, length, then the body.
    let mut body = Buf::default();
    let late = v.0 >= 16;
    match &fill {
        Fill::Solid(c) => {
            let mut inner = Buf::default();
            inner.bytes(&colour_props(v, *c)).zeros(8).bytes(&SCREEN);
            if !late {
                inner.zeros(8);
            }
            let inner = inner.done();
            body.u16(1)
                .u32(SINCE_13)
                .u32(inner.len() as u32)
                .bytes(&inner)
                .zeros(8);
        }
        Fill::Fountain(f) => {
            let mut inner = Buf::default();
            inner
                .u8(fountain_type(f.kind))
                .bytes(&[0, 0, 0, 1, 0x51, 0, 0, 0, 0])
                .bytes(&SCREEN)
                .u16((f.edge_pad * 100.0).round().clamp(0.0, 49.0) as u16)
                .i32((f.angle * 1_000_000.0).round() as i32)
                .i32((f.offset.x * 100.0).round() as i32)
                .i32((f.offset.y * 100.0).round() as i32)
                .zeros(2)
                .bytes(&[3, 0, 1, 0])
                .u8(50)
                .u8(0);
            let stops: Vec<_> = f.stops.iter().take(64).collect();
            inner.u16(stops.len() as u16);
            for s in stops {
                inner.u8(1).u32(12).bytes(&colour(s.color));
                if v.0 >= 15 {
                    inner.u8(7).u32(16).zeros(16);
                }
                inner
                    .zeros(5)
                    .u16((s.pos.clamp(0.0, 1.0) * 100.0).round() as u16);
            }
            inner.zeros(8);
            if late {
                // The fill's place and size relative to the object.
                inner.f64(0.0).f64(0.0).f64(1.0).f64(1.0);
            }
            let inner = inner.done();
            body.u16(2)
                .u32(SINCE_13)
                .u32(inner.len() as u32)
                .bytes(&inner)
                .zeros(8);
        }
        _ => {
            body.u16(0);
            if !late {
                body.zeros(8);
            }
        }
    }
    let body = body.done();
    b.u32(id).u32(SINCE_13).u32(body.len() as u32).bytes(&body);
    if late {
        b.zeros(8);
    }
    b.done()
}

/// Bytes of the identity matrix as six doubles.
fn identity_matrix() -> Vec<u8> {
    let mut b = Buf::default();
    for x in [1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        b.f64(x);
    }
    b.done()
}

/// `outl`: an outline record. Before version 13 the outline proper; from
/// 13 tagged records (3: a value of 5000000, 2: the colour, 1: the
/// outline proper) and 8 closing bytes.
fn outl(v: Ver, id: u32, s: Option<&Stroke>) -> Vec<u8> {
    let mut o = Buf::default();
    let (line_type, caps, join, width, stretch, angle, color, dash) = match s {
        None => (
            1u16,
            0u16,
            0u16,
            0i32,
            100u16,
            0i32,
            Color::BLACK,
            Vec::new(),
        ),
        Some(s) => {
            let mut t = 0x02u16;
            if s.scale_with_object {
                t |= 0x20;
            }
            if s.behind_fill {
                t |= 0x10;
            }
            let caps = match s.cap {
                LineCap::Butt => 0,
                LineCap::Round => 1,
                LineCap::Square => 2,
            };
            let join = match s.join {
                LineJoin::Miter => 0,
                LineJoin::Round => 1,
                LineJoin::Bevel => 2,
            };
            let width = if s.width <= Stroke::HAIRLINE + 1e-9 {
                0
            } else {
                units(s.width)
            };
            let dash: Vec<u16> = s
                .dash
                .iter()
                .take(10)
                .map(|d| d.round().clamp(0.0, 65535.0) as u16)
                .collect();
            (
                t,
                caps,
                join,
                width,
                (s.stretch.clamp(0.01, 1.0) * 100.0).round() as u16,
                (s.nib_angle * 1_000_000.0).round() as i32,
                s.color,
                dash,
            )
        }
    };
    o.u16(line_type).u16(caps).u16(join);
    if !v.tagged() {
        o.zeros(2);
    }
    o.i32(width).u16(stretch);
    if !v.tagged() {
        o.zeros(2);
    }
    o.i32(angle);
    if !v.tagged() {
        o.zeros(4);
    }
    o.bytes(&identity_matrix())
        .bytes(&colour(color))
        .zeros(8)
        .bytes(&SCREEN)
        .u16(dash.len() as u16);
    let dashes_at = o.len();
    for d in &dash {
        o.u16(*d);
    }
    // The count and ten dash slots fill 22 bytes; the two marker ids
    // follow (none), then 2 bytes we leave zero.
    o.pad_to(dashes_at + 20);
    o.u32(0).u32(0).zeros(2);
    let proper = o.done();
    let mut b = Buf::default();
    b.u32(id);
    if !v.tagged() {
        b.bytes(&proper).zeros(4);
        return b.done();
    }
    let mut proper = proper;
    proper.resize(130, 0);
    let col = colour_props(v, color);
    if v.0 == 15 {
        b.u32(5).u32(48).zeros(48);
    }
    b.u32(3)
        .u32(4)
        .u32(5_000_000)
        .u32(2)
        .u32(col.len() as u32)
        .bytes(&col)
        .u32(1)
        .u32(proper.len() as u32)
        .bytes(&proper)
        .zeros(8);
    b.done()
}

/// `trfd`: one transform record (type 8): the matrix rows (a c e; b d f),
/// translation in coordinate units.
fn trfd(v: Ver, m: Affine) -> Vec<u8> {
    let c = m.as_coeffs();
    let mut rec = Buf::default();
    if v.tagged() {
        rec.u32(SINCE_13).u32(0);
    }
    rec.u16(8).zeros(6);
    for x in [
        c[0],
        c[2],
        c[4] * UNITS_PER_MM,
        c[1],
        c[3],
        c[5] * UNITS_PER_MM,
    ] {
        rec.f64(if x.is_finite() { x } else { 0.0 });
    }
    if v.tagged() {
        rec.zeros(8);
    }
    let rec = rec.done();
    let start = 24u32;
    let end = start + rec.len() as u32;
    let mut b = Buf::default();
    b.u32(end)
        .u32(1)
        .u32(16)
        .u32(0xffff)
        .u32(start)
        .u32(end)
        .bytes(&rec);
    b.done()
}

/// `ftil`: the fill's transform, the identity.
fn ftil() -> Vec<u8> {
    identity_matrix()
}

/// `mcfg`: the document configuration. Versions before 21 get the page
/// size and the length files of the version have; from 21 the record has
/// the layout of those files with the fields that are understood (see
/// `docs/cdr-format.md`, Writing).
fn mcfg(v: Ver, w_mm: f64, h_mm: f64) -> Vec<u8> {
    if v.0 >= 21 {
        return mcfg_21(w_mm, h_mm);
    }
    let mut b = Buf::default();
    let size: usize = if v.0 >= 15 {
        8323
    } else if v.tagged() {
        8197
    } else {
        4088
    };
    if v.tagged() {
        b.u32(SINCE_13).u32(size as u32 - 8).zeros(4);
    } else if v.0 >= 9 {
        b.zeros(4);
    }
    b.i32(units(w_mm))
        .i32(units(h_mm))
        .u32(0x0f)
        .u32(1)
        .pad_to(size);
    b.done()
}

/// Write `bytes` into `rec` at `at` (records are built to their size
/// first, so the offsets are always inside).
fn put(rec: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(dst) = rec.get_mut(at..at + bytes.len()) {
        dst.copy_from_slice(bytes);
    }
}

/// The version 21 and 22 configuration record (7944 bytes): page size and
/// orientation, ruler origin, grid, units, duplicate offset and nudge,
/// drawing scales and the isometric drawing profile, with the values the
/// files of those versions have for a metric drawing. Text fields of the
/// record (a folder for web publishing) are left empty.
fn mcfg_21(w_mm: f64, h_mm: f64) -> Vec<u8> {
    const SIZE: usize = 7944;
    /// Units: 1 inch, 2 millimetre.
    const MM: u16 = 2;
    let mut r = vec![0u8; SIZE];
    let (w, h) = (units(w_mm), units(h_mm));
    put(&mut r, 0, &2100u32.to_le_bytes());
    put(&mut r, 4, &(SIZE as u32 - 8).to_le_bytes());
    put(&mut r, 12, &w.to_le_bytes());
    put(&mut r, 16, &h.to_le_bytes());
    // Paper preset (0: custom size) and orientation (1: landscape).
    put(&mut r, 20, &0u16.to_le_bytes());
    put(&mut r, 22, &u16::from(w > h).to_le_bytes());
    put(&mut r, 24, &1u32.to_le_bytes());
    put(&mut r, 34, &1u32.to_le_bytes());
    put(&mut r, 38, &1u32.to_le_bytes());
    // 1/8 inch, in every version 19 to 21 file seen.
    put(&mut r, 42, &31750u32.to_le_bytes());
    // Ruler origin: the page's bottom-left corner, from the page centre.
    put(&mut r, 670, &(-(w / 2)).to_le_bytes());
    put(&mut r, 674, &(-(h / 2)).to_le_bytes());
    // Grid: 4 a unit across and down, in millimetres.
    put(&mut r, 678, &4.0f64.to_le_bytes());
    put(&mut r, 686, &4.0f64.to_le_bytes());
    put(&mut r, 694, &MM.to_le_bytes());
    put(&mut r, 696, &1.0f64.to_le_bytes());
    // Horizontal and vertical units.
    put(&mut r, 704, &MM.to_le_bytes());
    put(&mut r, 706, &MM.to_le_bytes());
    put(&mut r, 708, &1u16.to_le_bytes());
    // A 46.5 inch square twice, with three flags between.
    for (at, x) in [
        (716, 11_811_024i32),
        (720, 11_811_024),
        (724, 1),
        (728, 1),
        (732, 1),
        (736, 11_811_024),
        (740, 11_811_024),
        (744, 1),
    ] {
        put(&mut r, at, &x.to_le_bytes());
    }
    put(&mut r, 760, &[0x01, 0x0c]);
    put(&mut r, 767, &[0x05]);
    put(&mut r, 777, &[0x07, 0x10]);
    put(&mut r, 811, &1u32.to_le_bytes());
    put(&mut r, 815, &1u32.to_le_bytes());
    put(&mut r, 819, &[1]);
    // Duplicate offset (5 mm across and down), nudge (0.1 mm), super
    // nudge times 2, micro nudge divided by 2.
    for (at, x) in [
        (827, 1i32),
        (831, 50_000),
        (835, 50_000),
        (839, 1_000),
        (843, 2),
        (847, 2),
        (851, 1),
    ] {
        put(&mut r, at, &x.to_le_bytes());
    }
    put(&mut r, 859, &MM.to_le_bytes());
    put(&mut r, 861, &12_700u16.to_le_bytes());
    for at in [869, 873, 877] {
        put(&mut r, at, &[1]);
    }
    put(&mut r, 885, &[0x01, 0x0c, 0, 0, 0, 0x05, 0, 0x05]);
    put(&mut r, 898, &[0xff, 0, 0, 0, 0x07, 0x10]);
    put(&mut r, 928, &[0x01, 0x0c, 0, 0, 0, 0x05, 0, 0x05]);
    put(&mut r, 942, &[0x64, 0, 0, 0x07, 0x10]);
    put(&mut r, 971, &[1]);
    put(&mut r, 1007, &[1]);
    // Four scales of 1, then 2, 0.2, 1 and a 4 by 4 grid.
    for k in 0..4 {
        put(&mut r, 1019 + 8 * k, &1.0f64.to_le_bytes());
    }
    put(&mut r, 1051, &2u32.to_le_bytes());
    put(&mut r, 1055, &0.2f64.to_le_bytes());
    put(&mut r, 1063, &1u32.to_le_bytes());
    put(&mut r, 1067, &4.0f64.to_le_bytes());
    put(&mut r, 1075, &4.0f64.to_le_bytes());
    put(&mut r, 1083, &[4, 1]);
    put(&mut r, 1100, &[1]);
    // The drawing profile: 12, three flags, three scales of 0.8165, the
    // axes at 330, 90 and 210 degrees (millionths) and its name.
    put(&mut r, 7360, &12.0f64.to_le_bytes());
    for at in [7368, 7372, 7376] {
        put(&mut r, at, &1u32.to_le_bytes());
    }
    for k in 0..3 {
        put(&mut r, 7380 + 8 * k, &0.8165f64.to_le_bytes());
    }
    for (k, a) in [330_000_000i32, 90_000_000, 210_000_000]
        .into_iter()
        .enumerate()
    {
        put(&mut r, 7404 + 4 * k, &a.to_le_bytes());
    }
    let name: Vec<u8> = "Isometric"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    put(&mut r, 7416, &name);
    r
}

/// `font`: id, character set, style flags, 10 bytes of classification,
/// the family name: 8-bit in a 74-byte field before version 12, UTF-16 in
/// a 296-byte record after (with the technology value 2 at 160).
fn font_record(v: Ver, id: u16, family: &str) -> Vec<u8> {
    let mut b = Buf::default();
    b.u16(id).u16(0).u32(0x40).zeros(10);
    if v.unicode() {
        let name: Vec<u16> = family.encode_utf16().take(31).collect();
        for u in &name {
            b.u16(*u);
        }
        b.pad_to(160).u32(2).pad_to(296);
    } else {
        let mut name =
            cp1252(family).unwrap_or_else(|| family.bytes().filter(|c| c.is_ascii()).collect());
        name.truncate(73);
        b.bytes(&name).pad_to(92);
    }
    b.done()
}

/// What the style table's styles use: the fill records of text (black) and
/// of graphics (none), the outline record (none) and the font.
#[derive(Debug, Clone, Copy)]
struct StyleUses {
    text_fill: u32,
    graphic_fill: u32,
    outline: u32,
    font: u16,
}

/// Decoration lines of the character settings (`set5`), as files of every
/// version carry them: (position, thickness) in millionths of the font
/// size, for underline (single thin, single thick, double), overline and
/// strikethrough; a double line has two entries.
const DECORATION_LINES: [(i32, i32); 12] = [
    (-90_909, 45_455),
    (-90_909, 90_909),
    (-90_909, 45_455),
    (-181_819, 45_455),
    (845_455, 45_455),
    (868_182, 90_909),
    (800_000, 45_455),
    (890_910, 45_455),
    (333_333, 45_455),
    (356_060, 90_909),
    (378_788, 45_455),
    (287_878, 45_455),
];

/// `stlt`: the style table. Its tables (fills, outlines, fonts,
/// alignments, spacings, character settings, then empty tab, bullet,
/// indent, hyphenation and drop cap tables, and from version 9 one set11
/// entry) number their entries in one sequence from 1, as files do, and
/// every style names an entry of each table it uses: a style per
/// alignment for artistic text (ids from `STYLE_ID`, 12 pt, 100 % line
/// spacing) and one for graphics (`GRAPHIC_STYLE_ID`). The version's
/// layout is the one `text::read_stlt` reads.
fn stlt(v: Ver, uses: StyleUses) -> Vec<u8> {
    let n = v.0;
    let mut next = 0u32;
    let mut id = || {
        next += 1;
        next
    };
    let (text_fill, graphic_fill) = (id(), id());
    let outline = id();
    let font = id();
    let aligns: Vec<u32> = STYLE_ALIGNS.iter().map(|_| id()).collect();
    let (interval, set5) = (id(), id());
    let set11 = id();
    let mut t = Buf::default();
    t.u32(STYLE_ALIGNS.len() as u32 + 1); // records
                                          // Fills: id, 4 unknown, the fill record, from 13 the fill's matrix.
    t.u32(2);
    for (entry, record) in [
        (text_fill, uses.text_fill),
        (graphic_fill, uses.graphic_fill),
    ] {
        t.u32(entry).u32(0).u32(record);
        if v.tagged() {
            t.bytes(&identity_matrix());
        }
    }
    // Outlines: id, 4 unknown, the outline record.
    t.u32(1).u32(outline).u32(0).u32(uses.outline);
    // Fonts: id, a mask (12 bytes before 10, 20 after), then the font,
    // size and style of the three scripts (Latin, Asian, other), all in
    // the one font: font id u16 and character set u16, two more font
    // references, sizes, style flags (one before 10).
    let size = (12.0 * 254000.0 / 72.0) as i32;
    let script_font = u32::from(uses.font);
    let mask = if v.redirects() { 0xff } else { 0xffff };
    t.u32(1).u32(font).u32(0).u32(mask).u32(mask);
    if n >= 10 {
        t.u32(mask).u32(if v.redirects() { u32::MAX } else { 1 });
    }
    t.u16(uses.font).u16(0).u32(script_font).u32(script_font);
    t.i32(size).i32(size).i32(size).u32(0x40);
    if n >= 10 {
        t.u32(0x40).u32(0x40);
    }
    // Alignments: id, 4 unknown, value.
    t.u32(STYLE_ALIGNS.len() as u32);
    for (entry, (_, code)) in aligns.iter().zip(STYLE_ALIGNS.iter()) {
        t.u32(*entry).u32(0).u32(*code);
    }
    // Spacings: id, 0, a mask (0xffff, from 16 0x3ff), character spacing
    // 0, then 0, 100 %, the line spacing (100 %), 100 %, 0, 200 %, 60 %,
    // 300 % and 0 in millionths: the values every sample file has, of
    // which only the character and line spacings are understood.
    t.u32(1).u32(interval).u32(0);
    t.u32(if v.redirects() { 0x3ff } else { 0xffff });
    t.u32(0).u32(0).u32(1_000_000).u32(1_000_000).u32(1_000_000);
    t.u32(0).u32(2_000_000).u32(600_000).u32(3_000_000).u32(0);
    // Character settings: id, 0, then each decoration line as 0,
    // position, thickness.
    t.u32(1).u32(set5).u32(0);
    for (position, thickness) in DECORATION_LINES {
        t.u32(0).i32(position).i32(thickness);
    }
    // Tabs, bullets, indents, hyphenation, drop caps: none; set11 from 9.
    for _ in 0..5 {
        t.u32(0);
    }
    if n >= 9 {
        t.u32(1).u32(set11).u32(0).u32(0);
    }
    // The records: sections (1: fill and outline; 2: also the character
    // and paragraph settings), id, no parent, 8 unknown, the name (none
    // from 16, as files have it), then the entries they use.
    let name = |t: &mut Buf, s: &str| {
        let s = if v.redirects() { "" } else { s };
        t.u32(s.chars().count() as u32);
        if v.unicode() {
            t.bytes(&utf16(s));
        } else {
            t.bytes(s.as_bytes());
        }
    };
    let labels = ["Default", "Centred", "Right", "Justified"];
    for (i, (label, align)) in labels.iter().zip(&aligns).enumerate() {
        t.u32(2).u32(STYLE_ID + i as u32).u32(0).zeros(8);
        name(&mut t, label);
        t.u32(text_fill).u32(outline);
        t.u32(font).u32(*align).u32(interval).u32(set5);
        if n >= 9 {
            t.u32(set11);
        }
    }
    t.u32(1).u32(GRAPHIC_STYLE_ID).u32(0).zeros(8);
    name(&mut t, "Default Graphic");
    t.u32(graphic_fill).u32(outline);
    t.done()
}

/// `bmp `: a 24-bit image: id, a file-like header ("UI" block size, the
/// "RI" block with its size and pixel offset), colour model 1, size,
/// bits, row stride, pixel bytes, resolution, then rows bottom-up in
/// B G R order padded to 4 bytes. Transparent pixels are put over white.
fn bmp_record(id: u32, png: &[u8], size_mm: (f64, f64)) -> Option<Vec<u8>> {
    let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
    let (w, h) = (pm.width(), pm.height());
    if w == 0 || h == 0 {
        return None;
    }
    let stride = (w as usize * 24).div_ceil(32) * 4;
    let mut pixels = vec![0u8; stride * h as usize];
    let src = pm.pixels();
    for row in 0..h as usize {
        let Some(dst) = pixels.get_mut((h as usize - 1 - row) * stride..) else {
            continue;
        };
        for x in 0..w as usize {
            let Some(p) = src.get(row * w as usize + x).map(|p| p.demultiply()) else {
                continue;
            };
            let a = p.alpha() as u32;
            let blend = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
            if let Some(px) = dst.get_mut(x * 3..x * 3 + 3) {
                px.copy_from_slice(&[blend(p.blue()), blend(p.green()), blend(p.red())]);
            }
        }
    }
    let per_km = |px: u32, mm: f64| {
        if mm > 1e-6 {
            (px as f64 / mm * 1_000_000.0)
                .round()
                .clamp(1.0, u32::MAX as f64) as u32
        } else {
            2_834_646
        }
    };
    let len = 4 + 50 + 32 + 32 + pixels.len();
    let mut b = Buf::default();
    b.u32(id)
        .u32(1)
        .u32(0x4955)
        .u32((len - 8) as u32)
        .u32(0)
        .u32(32)
        .zeros(8)
        .u32(2)
        .u32(255)
        .u16(0x4952)
        .u32((len - 40) as u32)
        .u32(0)
        .u32((len - pixels.len() - 40) as u32);
    b.u32(1)
        .u32(1)
        .u32(w)
        .u32(h)
        .u32(1)
        .u32(24)
        .u32(stride as u32)
        .u32(pixels.len() as u32);
    b.u32(0)
        .u32(per_km(w, size_mm.0))
        .u32(per_km(h, size_mm.1))
        .u32(0)
        .u32(64)
        .zeros(12);
    b.bytes(&pixels);
    Some(b.done())
}

/// `loda`: header (length, argument count, offsets table, types table,
/// object type), the offsets table with the end marker, the types table
/// in reverse order, then the argument bodies back to back. Arguments go
/// highest type first, as files order them.
fn loda(kind: u32, mut args: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    args.sort_by_key(|a| std::cmp::Reverse(a.0));
    let n = args.len();
    let start_args = 20usize;
    let start_types = start_args + (n + 1) * 4;
    let mut pos = start_types + n * 4;
    let mut offsets = Vec::with_capacity(n + 1);
    for (_, b) in &args {
        offsets.push(pos as u32);
        pos += b.len();
    }
    offsets.push(pos as u32);
    let mut out = Buf::default();
    out.u32(pos as u32)
        .u32(n as u32)
        .u32(start_args as u32)
        .u32(start_types as u32)
        .u32(kind);
    for o in &offsets {
        out.u32(*o);
    }
    for (t, _) in args.iter().rev() {
        out.u32(*t);
    }
    for (_, b) in &args {
        out.bytes(b);
    }
    out.done()
}

/// Opacity argument: 10 unknown bytes (14 from version 13), then the
/// transparency in thousandths.
fn opacity_arg(v: Ver, opacity: f64) -> Vec<u8> {
    let mut b = Buf::default();
    b.zeros(if v.tagged() { 14 } else { 10 })
        .u16(((1.0 - opacity.clamp(0.0, 1.0)) * 1000.0).round() as u16);
    b.done()
}

/// Points and node bytes of a path in local units: bits 7-6 the node kind
/// (00 move, 01 line, 10 curve end, 11 control point), bits 5-4 the
/// node's type (0 cusp, 1 smooth, 2 symmetrical), bit 3 closes the subpath.
fn curve_coords(path: &BezPath) -> Option<Vec<u8>> {
    curve_coords_typed(path, &Default::default())
}

/// [`curve_coords`] for a curve with node types chosen for it.
fn curve_coords_typed(
    path: &BezPath,
    chosen: &std::collections::BTreeMap<usize, tracedraw_core::nodes::NodeType>,
) -> Option<Vec<u8>> {
    use tracedraw_core::nodes::{self, NodeType};
    let continuity: std::collections::BTreeMap<usize, u8> = nodes::nodes(path)
        .into_iter()
        .map(|n| {
            let bits = match nodes::node_type_of_with(&n, chosen) {
                NodeType::Cusp => 0x00,
                NodeType::Smooth => 0x10,
                NodeType::Symmetrical => 0x20,
            };
            (n.index, bits)
        })
        .collect();
    let node_bits = |i: usize| continuity.get(&i).copied().unwrap_or(0);
    let mut pts: Vec<(i32, i32)> = Vec::new();
    let mut types: Vec<u8> = Vec::new();
    let mut last_start = 0usize;
    let mut last = Point::ZERO;
    for (i, el) in path.elements().iter().enumerate() {
        match el {
            PathEl::MoveTo(p) => {
                pts.push((units(p.x), units(p.y)));
                types.push(node_bits(i));
                last_start = pts.len() - 1;
                last = *p;
            }
            PathEl::LineTo(p) => {
                pts.push((units(p.x), units(p.y)));
                types.push(0x40 | node_bits(i));
                last = *p;
            }
            PathEl::QuadTo(c, p) => {
                // Raised to a cubic.
                let c1 = last + (*c - last) * (2.0 / 3.0);
                let c2 = *p + (*c - *p) * (2.0 / 3.0);
                pts.push((units(c1.x), units(c1.y)));
                types.push(0xc0);
                pts.push((units(c2.x), units(c2.y)));
                types.push(0xc0);
                pts.push((units(p.x), units(p.y)));
                types.push(0x80 | node_bits(i));
                last = *p;
            }
            PathEl::CurveTo(a, b, p) => {
                pts.push((units(a.x), units(a.y)));
                types.push(0xc0);
                pts.push((units(b.x), units(b.y)));
                types.push(0xc0);
                pts.push((units(p.x), units(p.y)));
                types.push(0x80 | node_bits(i));
                last = *p;
            }
            PathEl::ClosePath => {
                // A closing line back to the subpath start, flagged.
                let s = pts.get(last_start).copied().unwrap_or((0, 0));
                pts.push(s);
                types.push(0x48);
            }
        }
    }
    if pts.is_empty() {
        return None;
    }
    let mut out = Buf::default();
    out.u32(pts.len() as u32);
    for (x, y) in &pts {
        out.i32(*x).i32(*y);
    }
    out.bytes(&types);
    Some(out.done())
}

/// A box as `bbox` stores it: top left, then bottom right (file units,
/// y up).
fn box_bytes(r: Rect) -> Vec<u8> {
    let mut b = Buf::default();
    b.i32(units(r.x0))
        .i32(units(r.y1))
        .i32(units(r.x1))
        .i32(units(r.y0));
    b.done()
}

/// `obbx`: the box, then the box with the outline.
fn obbx_bytes(r: Rect, outline: f64) -> Vec<u8> {
    let mut b = box_bytes(r);
    b.extend(box_bytes(r.inflate(outline, outline)));
    b
}

/// The style string of an object (versions 16+): its fill, outline and
/// transparency.
fn object_style_json(fill: &Fill, stroke: Option<&Stroke>, opacity: f64) -> String {
    let fill_part = match fill_colour(fill) {
        Some(c) if !matches!(fill, Fill::None) => {
            format!(r#"{{"type":"1","primaryColor":"{}"}}"#, colour_spec(c))
        }
        _ => r#"{"type":"0"}"#.to_string(),
    };
    let outline_part = match stroke {
        Some(s) => format!(
            r#"{{"width":"{}","color":"{}"}}"#,
            if s.width <= Stroke::HAIRLINE + 1e-9 {
                0
            } else {
                units(s.width)
            },
            colour_spec(s.color)
        ),
        None => r#"{"type":"1","width":"0"}"#.to_string(),
    };
    let transparency = if opacity < 0.999 {
        format!(
            r#"{{"type":"1","uniformTransparency":"{}"}}"#,
            ((1.0 - opacity.clamp(0.0, 1.0)) * 1000.0).round() / 1000.0
        )
    } else {
        "{}".to_string()
    };
    format!(r#"{{"fill":{fill_part},"outline":{outline_part},"transparency":{transparency}}}"#)
}

/// The style codes of the font tables: one bit per weight and slant
/// (0x40 regular, 0x80 italic, 0x1000 bold, 0x2000 bold italic), single
/// underline in bits 18 to 20, single strike-through in bits 24 to 26.
fn style_flags(sp: &TextSpan) -> u32 {
    let mut f = match (sp.bold, sp.italic) {
        (false, false) => 0x40,
        (false, true) => 0x80,
        (true, false) => 0x1000,
        (true, true) => 0x2000,
    };
    if sp.underline {
        f |= 1 << 18;
    }
    if sp.strikethrough {
        f |= 1 << 24;
    }
    f
}

/// The style string of a run (versions 16+): its font, size, weight,
/// slant, lines and colour.
fn run_style_json(sp: &TextSpan, fill: Option<&Fill>) -> String {
    format!("{{{}}}", character_json(sp, fill))
}

/// The `character` member of a style string.
fn character_json(sp: &TextSpan, fill: Option<&Fill>) -> String {
    let weight = if sp.bold { 4096 } else { 64 };
    let size = (sp.size_pt * 254000.0 / 72.0).round() as i64;
    let family = sp.font_family.replace('\\', "\\\\").replace('"', "\\\"");
    let font = format!(
        r#"{{"font":"{family}","size":"{size}","weight":"{weight}","italic":"{}","underline":"{}","strikeout":"{}"}}"#,
        u8::from(sp.italic),
        u8::from(sp.underline),
        u8::from(sp.strikethrough)
    );
    let fill = match fill.and_then(fill_colour) {
        Some(c) => format!(
            r#""fill":{{"type":"1","primaryColor":"{}"}},"#,
            colour_spec(c)
        ),
        None => String::new(),
    };
    format!(r#""character":{{{fill}"latin":{font},"RTL":{font},"farEast":{font}}}"#)
}

/// The `justify` value of a style string: 1 left, 2 centre, 3 right, 4
/// full justify (5 forced).
fn justify_code(align: TextAlign) -> u32 {
    match align {
        TextAlign::Left => 1,
        TextAlign::Center => 2,
        TextAlign::Right => 3,
        TextAlign::Justify => 4,
    }
}

// ---- the writer ----------------------------------------------------------

struct Writer<'a> {
    v: Ver,
    symbols: &'a [Symbol],
    fills: Vec<Vec<u8>>,
    fill_ids: HashMap<String, u32>,
    outlines: Vec<Vec<u8>>,
    outline_ids: HashMap<String, u32>,
    fonts: Vec<(u16, String)>,
    bitmaps: Vec<Vec<u8>>,
    /// Static ids (`usdn`), object ids (argument 100), next fill, outline
    /// and bitmap ids.
    next_usdn: u32,
    next_object: u32,
    next_fill: u32,
    next_outline: u32,
    next_bitmap: u32,
    /// The next text object's `spnd` number.
    next_spnd: u32,
    guid_seed: u64,
    guids: u64,
    /// The GUIDs of the master page's guides, desktop and grid layers,
    /// which each page's entries for them repeat.
    master_guids: [[u8; 16]; 3],
    has_text: bool,
}

/// The master page's layers, in the order of `Writer::master_guids`.
#[derive(Debug, Clone, Copy)]
enum MasterLayer {
    Guides,
    Desktop,
    Grid,
}

impl MasterLayer {
    fn flags(self) -> [u8; 4] {
        match self {
            MasterLayer::Guides => [0x0a, 0, 1, 0x98],
            MasterLayer::Desktop => [0x08, 0, 1, 0x98],
            MasterLayer::Grid => [0x1a, 1, 1, 0x98],
        }
    }

    fn kind(self) -> u32 {
        match self {
            MasterLayer::Guides => LAYER_GUIDES,
            MasterLayer::Desktop => LAYER_DESKTOP,
            MasterLayer::Grid => LAYER_GRID,
        }
    }

    fn name(self) -> &'static str {
        match self {
            MasterLayer::Guides => "Guides",
            MasterLayer::Desktop => "Desktop",
            MasterLayer::Grid => "Document Grid",
        }
    }
}

/// Where a written object sits: its box and the box with its outline.
struct Placed {
    nodes: Vec<Node>,
    bounds: Option<Rect>,
}

impl<'a> Writer<'a> {
    fn new(v: Ver, symbols: &'a [Symbol], title: &str) -> Self {
        let seed = title.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
        });
        let master_guids = [guid(seed, 1), guid(seed, 2), guid(seed, 3)];
        Writer {
            v,
            symbols,
            fills: Vec::new(),
            fill_ids: HashMap::new(),
            outlines: Vec::new(),
            outline_ids: HashMap::new(),
            fonts: Vec::new(),
            bitmaps: Vec::new(),
            next_usdn: 1,
            next_object: 1,
            next_fill: 1,
            next_outline: 1,
            next_bitmap: 1,
            next_spnd: 1,
            guid_seed: seed,
            guids: master_guids.len() as u64,
            master_guids,
            has_text: false,
        }
    }

    fn guid(&mut self) -> [u8; 16] {
        self.guids += 1;
        guid(self.guid_seed, self.guids)
    }

    fn fill_id(&mut self, fill: &Fill) -> u32 {
        let key = format!("{fill:?}");
        if let Some(id) = self.fill_ids.get(&key) {
            return *id;
        }
        let id = self.next_fill;
        self.next_fill += 1;
        self.fills.push(fild(self.v, id, fill));
        self.fill_ids.insert(key, id);
        id
    }

    fn outline_id(&mut self, stroke: Option<&Stroke>) -> u32 {
        let key = format!("{stroke:?}");
        if let Some(id) = self.outline_ids.get(&key) {
            return *id;
        }
        let id = self.next_outline;
        self.next_outline += 1;
        self.outlines.push(outl(self.v, id, stroke));
        self.outline_ids.insert(key, id);
        id
    }

    fn font_id(&mut self, family: &str) -> u16 {
        if let Some((id, _)) = self.fonts.iter().find(|(_, f)| f == family) {
            return *id;
        }
        let id = self.fonts.len() as u16 + 1;
        self.fonts.push((id, family.to_string()));
        id
    }

    fn bitmap_id(&mut self, png: &[u8], size_mm: (f64, f64)) -> Option<u32> {
        let id = self.next_bitmap;
        let rec = bmp_record(id, png, size_mm)?;
        self.next_bitmap += 1;
        self.bitmaps.push(rec);
        Some(id)
    }

    /// The whole file: version chunks, bitmaps, the document tables, the
    /// master page and every page.
    fn document(&mut self, doc: &Document) -> Vec<Node> {
        let first = doc
            .pages
            .first()
            .map(|p| p.size)
            .unwrap_or(tracedraw_core::geometry::Size::new(210.0, 297.0));
        // Pages first: they fill the tables.
        let mut pages = Vec::new();
        let mut content: Option<Rect> = None;
        for page in &doc.pages {
            let (node, b) = self.page(page);
            if content.is_none() {
                content = b;
            }
            pages.push(node);
        }
        let v = self.v;
        let mut out = Vec::new();
        if v.redirects() {
            let mut fver = Buf::default();
            fver.u16(v.0 * 100).u16(1).u16(0).u16(v.0);
            out.push(chunk(b"fver", fver.done()));
        }
        out.push(chunk(b"vrsn", (v.0 * 100).to_le_bytes().to_vec()));
        if !self.bitmaps.is_empty() {
            let bmps = std::mem::take(&mut self.bitmaps)
                .into_iter()
                .map(|b| chunk(b"bmp ", b))
                .collect();
            out.push(list(b"bmpt", bmps));
        }
        let mut docl = Vec::new();
        if !v.redirects() {
            docl.push(chunk(b"pfrd", vec![0; if v.tagged() { 2 } else { 1 }]));
        }
        docl.push(chunk(b"mcfg", mcfg(v, first.width, first.height)));
        let content = content.unwrap_or(Rect::ZERO);
        let centre = Point::new(first.width / 2.0, first.height / 2.0);
        docl.push(chunk(b"ptrt", box_bytes(content - centre.to_vec2())));
        // The styles name a font, so the font table is never empty.
        if self.fonts.is_empty() {
            self.font_id("Arial");
        }
        if !self.fonts.is_empty() {
            let fonts = self
                .fonts
                .iter()
                .map(|(id, f)| chunk(b"font", font_record(v, *id, f)))
                .collect();
            docl.push(list(b"fntt", fonts));
        }
        // Every object refers to a fill and an outline, and the styles to
        // the fills of text (black) and graphics (none) and to no outline.
        let uses = StyleUses {
            text_fill: self.fill_id(&Fill::Solid(Color::BLACK)),
            graphic_fill: self.fill_id(&Fill::None),
            outline: self.outline_id(None),
            font: self.fonts.first().map(|(id, _)| *id).unwrap_or(1),
        };
        let fills = std::mem::take(&mut self.fills)
            .into_iter()
            .map(|f| list(b"filc", vec![chunk(b"fild", f)]))
            .collect();
        docl.push(list(b"filt", fills));
        let outlines = std::mem::take(&mut self.outlines)
            .into_iter()
            .map(|o| chunk(b"outl", o))
            .collect();
        docl.push(list(b"otlt", outlines));
        let styles = stlt(v, uses);
        if v.redirects() {
            docl.push(chunk(b"stlt", styles));
        } else {
            // Before version 16 the style table is the body of a list.
            docl.push(Node::RawList(*b"stlt", styles));
        }
        out.push(list(b"doc ", docl));
        out.push(self.master_page());
        out.extend(pages);
        out
    }

    /// A layer list: its flags, its properties (kind, a GUID from version
    /// 14, the colour and, unless it is a page's entry for a master layer,
    /// the name) and its objects.
    fn layer_entry(
        &mut self,
        flags: [u8; 4],
        kind: u32,
        name: Option<&str>,
        guid: [u8; 16],
        objects: Vec<Node>,
    ) -> Node {
        let v = self.v;
        let colour_bytes = match kind {
            LAYER_GUIDES => Color::rgb8(0, 0, 255),
            LAYER_GRID => Color::rgb8(194, 194, 193),
            _ => Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
        };
        let mut args = Vec::new();
        if let Some(name) = name {
            args.push((ARG_NAME, name_bytes(v, name)));
        }
        if v.tagged() {
            args.push((ARG_LAYER_COLOR, colour_props(v, colour_bytes)));
        } else {
            args.push((ARG_LAYER_COLOR, colour(colour_bytes).to_vec()));
        }
        if v.0 >= 17 {
            args.push((ARG_PROPS, vec![0; 4]));
            if kind == LAYER_GUIDES {
                args.push((ARG_GUIDES_MODE, 2u32.to_le_bytes().to_vec()));
            }
        }
        if v.zip() {
            args.push((ARG_GUID, guid.to_vec()));
        }
        let mut children = vec![
            chunk(b"flgs", flags.to_vec()),
            list(b"lgob", vec![chunk(b"loda", loda(kind, args))]),
        ];
        children.extend(objects);
        list(b"layr", children)
    }

    /// A layer of its own, with a new GUID.
    fn layer(&mut self, flags: [u8; 4], kind: u32, name: &str, objects: Vec<Node>) -> Node {
        let guid = self.guid();
        self.layer_entry(flags, kind, Some(name), guid, objects)
    }

    /// A layer of the master page (`page` false), or a page's entry for it:
    /// the same flags and GUID, and from version 14 no name.
    fn master_layer(&mut self, which: MasterLayer, page: bool) -> Node {
        let guid = self.master_guids[which as usize];
        let name = (!page || !self.v.zip()).then(|| which.name());
        self.layer_entry(which.flags(), which.kind(), name, guid, Vec::new())
    }

    /// The master page: guides, desktop and grid layers, no objects.
    fn master_page(&mut self) -> Node {
        let v = self.v;
        let mut children = vec![chunk(
            b"flgs",
            vec![if v.zip() { 0x40 } else { 0 }, 0, 1, 0x90],
        )];
        let unit = 1.0 / UNITS_PER_MM;
        let tiny = Rect::new(-2.0 * unit, -2.0 * unit, 2.0 * unit, 2.0 * unit);
        if v.zip() {
            children.push(chunk(b"bbox", box_bytes(tiny)));
            children.push(chunk(b"obbx", obbx_bytes(tiny, 0.0)));
        } else {
            children.push(chunk(b"bbox", vec![0; 16]));
        }
        let layers = vec![
            self.master_layer(MasterLayer::Guides, false),
            self.master_layer(MasterLayer::Desktop, false),
            self.master_layer(MasterLayer::Grid, false),
        ];
        children.push(list(b"gobj", layers));
        if v.redirects() {
            children.push(list(b"lgob", vec![chunk(b"loda", loda(0, Vec::new()))]));
        }
        list(b"page", children)
    }

    /// A page: its flags, the box of its objects, its entries for the
    /// master's guides and desktop layers, its own guides layer (from
    /// version 14), its layers and its entry for the master's grid layer.
    fn page(&mut self, page: &tracedraw_core::document::Page) -> (Node, Option<Rect>) {
        let v = self.v;
        let centre = Point::new(page.size.width / 2.0, page.size.height / 2.0);
        let mut layers = vec![
            self.master_layer(MasterLayer::Guides, true),
            self.master_layer(MasterLayer::Desktop, true),
        ];
        if v.zip() {
            layers.push(self.layer([0x0a, 0, 0, 0x98], LAYER_GUIDES, "Guides", Vec::new()));
        }
        let mut content: Option<Rect> = None;
        for layer in &page.layers {
            let objects = if layer.visible {
                let placed = self.objects(&layer.shapes, centre, 0);
                if let Some(b) = placed.bounds {
                    content = Some(content.map_or(b, |c| c.union(b)));
                }
                placed.nodes
            } else {
                Vec::new()
            };
            layers.push(self.layer([0, 0, 0, 0x98], LAYER_PLAIN, &layer.name, objects));
        }
        layers.push(self.master_layer(MasterLayer::Grid, true));
        let page_box = content.map(|c| c - centre.to_vec2()).unwrap_or(Rect::ZERO);
        let mut children = vec![chunk(
            b"flgs",
            vec![if v.zip() { 0x40 } else { 0 }, 0, 0, 0x90],
        )];
        children.push(chunk(b"bbox", box_bytes(page_box)));
        children.push(chunk(b"obbx", obbx_bytes(page_box, 0.0)));
        children.push(list(b"gobj", layers));
        (list(b"page", children), content)
    }

    /// Objects of a layer or group, front to back as files store them.
    fn objects(&mut self, shapes: &[Shape], centre: Point, depth: usize) -> Placed {
        let mut nodes = Vec::new();
        let mut bounds: Option<Rect> = None;
        if depth > 60 {
            return Placed { nodes, bounds };
        }
        for s in shapes.iter().rev() {
            let p = self.object(s, centre, depth);
            if let Some(b) = p.bounds {
                bounds = Some(bounds.map_or(b, |c| c.union(b)));
            }
            nodes.extend(p.nodes);
        }
        Placed { nodes, bounds }
    }

    /// The common head of an object or group list: flags, boxes and the
    /// static id.
    fn head(&mut self, flags: [u8; 4], page_box: Rect, outline: f64, centre: Point) -> Vec<Node> {
        let b = page_box - centre.to_vec2();
        let usdn = self.next_usdn;
        self.next_usdn += 1;
        vec![
            chunk(b"flgs", flags.to_vec()),
            chunk(b"bbox", box_bytes(b)),
            chunk(b"obbx", obbx_bytes(b, outline)),
            chunk(b"usdn", usdn.to_le_bytes().to_vec()),
        ]
    }

    fn group(&mut self, children: &[Shape], centre: Point, depth: usize) -> Placed {
        let inner = self.objects(children, centre, depth + 1);
        let Some(b) = inner.bounds else {
            return Placed {
                nodes: Vec::new(),
                bounds: None,
            };
        };
        let id = self.next_object;
        self.next_object += 1;
        let mut body = self.head([0, 0, 0, 0x10], b, 0.0, centre);
        body.push(list(
            b"lgob",
            vec![
                chunk(b"loda", loda(0, vec![(ARG_ID, id.to_le_bytes().to_vec())])),
                list(
                    b"trfl",
                    vec![chunk(b"trfd", trfd(self.v, Affine::IDENTITY))],
                ),
            ],
        ));
        body.extend(inner.nodes);
        Placed {
            nodes: vec![list(b"grp ", body)],
            bounds: Some(b),
        }
    }

    /// A bitmap object: image rectangle, image id and crop path (the whole
    /// rectangle, or `crop` in the bitmap's local space).
    fn bitmap_object(&mut self, s: &Shape, centre: Point, crop: Option<&BezPath>) -> Placed {
        let ShapeKind::Bitmap { rect, png, .. } = &s.kind else {
            return Placed {
                nodes: Vec::new(),
                bounds: None,
            };
        };
        let (sx, sy) = {
            let c = s.transform.as_coeffs();
            (c[0].hypot(c[1]), c[2].hypot(c[3]))
        };
        let Some(image) = self.bitmap_id(png, (rect.width() * sx, rect.height() * sy)) else {
            return Placed {
                nodes: Vec::new(),
                bounds: None,
            };
        };
        let v = self.v;
        let mut c = Buf::default();
        c.i32(units(rect.x0))
            .i32(units(rect.y0))
            .i32(units(rect.x1))
            .i32(units(rect.y1))
            .zeros(32)
            .u32(image)
            .zeros(if v.0 == 8 { 12 } else { 20 });
        let path_bytes = match crop {
            Some(p) => curve_coords(p),
            None => curve_coords(&rect.to_path(0.01)),
        };
        c.bytes(&path_bytes.unwrap_or_else(|| 0u32.to_le_bytes().to_vec()));
        let page_box = match crop {
            Some(p) => (s.transform * p.clone()).bounding_box(),
            None => (s.transform * rect.to_path(0.01)).bounding_box(),
        };
        self.place(
            s,
            OBJ_BITMAP,
            c.done(),
            &Fill::None,
            s.transform,
            page_box,
            None,
            centre,
        )
    }

    /// An object list: head, attributes with the transform and, for text,
    /// the text record (and, from version 16, the character settings for
    /// the object's style string).
    #[allow(clippy::too_many_arguments)]
    fn place(
        &mut self,
        s: &Shape,
        kind: u32,
        coords: Vec<u8>,
        fill: &Fill,
        transform: Affine,
        page_box: Rect,
        text: Option<(Vec<u8>, String, u32)>,
        centre: Point,
    ) -> Placed {
        let v = self.v;
        let fill_id = self.fill_id(fill);
        let outline_id = self.outline_id(s.stroke.as_ref());
        let id = self.next_object;
        self.next_object += 1;
        let mut args: Vec<(u32, Vec<u8>)> = vec![
            (ARG_COORDS, coords),
            (ARG_FILL, fill_id.to_le_bytes().to_vec()),
            (ARG_OUTLINE, outline_id.to_le_bytes().to_vec()),
            (ARG_ID, id.to_le_bytes().to_vec()),
        ];
        if v.style_strings() {
            let mut json = object_style_json(fill, s.stroke.as_ref(), s.opacity);
            if let Some((_, character, _)) = &text {
                // Text objects keep their character settings there too.
                json.pop();
                json.push(',');
                json.push_str(character);
                json.push('}');
                args.push((ARG_TEXT_LINK, 1u32.to_le_bytes().to_vec()));
            }
            let mut a = Buf::default();
            a.u32(1).bytes(&style_string(v, &json));
            args.push((ARG_STYLE_STRING, a.done()));
        } else {
            // Text refers to the style of its alignment, as its paragraphs;
            // graphics to the graphic style.
            let style = text
                .as_ref()
                .map_or(GRAPHIC_STYLE_ID, |(_, _, style)| *style);
            args.push((ARG_STYLE, style.to_le_bytes().to_vec()));
        }
        if v.zip() {
            args.push((ARG_PROPS, vec![0; 4]));
        }
        if let Some(n) = &s.name {
            args.push((ARG_NAME, name_bytes(v, n)));
        }
        if s.opacity < 0.999 {
            args.push((ARG_OPACITY, opacity_arg(v, s.opacity)));
        }
        let outline = s.stroke.as_ref().map(|st| st.width / 2.0).unwrap_or(0.0);
        // Text objects start with a number of their own (`spnd`: 1, 2, ...
        // in the order written, as files number them) and before version
        // 18 are flagged 9 instead of 8.
        let mut body = Vec::new();
        let mut flags = [0, 2, 0, 8];
        if text.is_some() {
            body.push(chunk(b"spnd", self.next_spnd.to_le_bytes().to_vec()));
            self.next_spnd += 1;
            if v.0 < 18 {
                flags[3] = 9;
            }
        }
        body.extend(self.head(flags, page_box, outline, centre));
        let recenter = Affine::translate(-centre.to_vec2());
        body.push(list(
            b"lgob",
            vec![
                chunk(b"loda", loda(kind, args)),
                chunk(b"ftil", ftil()),
                list(b"trfl", vec![chunk(b"trfd", trfd(v, recenter * transform))]),
            ],
        ));
        if let Some((t, _, _)) = text {
            body.push(chunk(b"txsm", t));
        }
        Placed {
            nodes: vec![list(b"obj ", body)],
            bounds: Some(page_box),
        }
    }

    fn object(&mut self, s: &Shape, centre: Point, depth: usize) -> Placed {
        let none = || Placed {
            nodes: Vec::new(),
            bounds: None,
        };
        if !s.visible {
            return none();
        }
        if !s.effects.is_empty() {
            let ev = tracedraw_core::live::evaluate(s);
            let mut all: Vec<Shape> = ev.below.clone();
            all.push(ev.main.clone());
            all.extend(ev.above.iter().cloned());
            for x in all.iter_mut() {
                x.effects.clear();
            }
            return self.objects(&all, centre, depth + 1);
        }
        let v = self.v;
        match &s.kind {
            ShapeKind::Group { children } => {
                let kids: Vec<Shape> = children
                    .iter()
                    .map(|c| {
                        let mut c = c.clone();
                        c.absorb(s.transform);
                        c
                    })
                    .collect();
                return self.group(&kids, centre, depth);
            }
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. } => {
                let kids: Vec<Shape> = s
                    .expand(self.symbols)
                    .into_iter()
                    .map(|mut c| {
                        c.absorb(s.transform);
                        c
                    })
                    .collect();
                return self.group(&kids, centre, depth);
            }
            ShapeKind::ClipFrame { frame, contents }
                if contents.len() == 1
                    && matches!(contents[0].kind, ShapeKind::Bitmap { .. })
                    && contents[0].effects.is_empty() =>
            {
                // A clipped bitmap is a bitmap object with a crop path.
                let bm = &contents[0];
                let clip = bm.transform.inverse() * frame.page_path();
                let mut b = bm.clone();
                b.transform = s.transform * bm.transform;
                return self.bitmap_object(&b, centre, Some(&clip));
            }
            ShapeKind::ClipFrame { frame, contents } => {
                let mut kids = Vec::new();
                for c in contents {
                    let mut c = c.clone();
                    c.absorb(s.transform);
                    kids.push(c);
                }
                let mut f = (**frame).clone();
                f.absorb(s.transform);
                f.fill = Fill::None;
                kids.push(f);
                return self.group(&kids, centre, depth);
            }
            ShapeKind::Bitmap { .. } => return self.bitmap_object(s, centre, None),
            _ => {}
        }
        let page_box = s.bounds();
        // Rectangles one round radius describes stay rectangles (before
        // version 15); other corners go out as the curve they draw.
        let plain = s.plain_rect().filter(|_| v.native_rects());
        match &s.kind {
            ShapeKind::Rect { .. } if plain.is_some() => {
                // Files draw rectangles from their top left corner down
                // (a negative height), which sets the way corners turn.
                let (rect, radius) = plain.unwrap_or_default();
                let t = s.transform * Affine::translate((rect.x0, rect.y1));
                let r = units(radius);
                let mut c = Buf::default();
                c.i32(units(rect.width())).i32(-units(rect.height())).i32(r);
                if v.0 >= 9 {
                    c.i32(r).i32(r).i32(r);
                }
                self.place(s, OBJ_RECT, c.done(), &s.fill, t, page_box, None, centre)
            }
            ShapeKind::Ellipse { rect, arc } => {
                let t = s.transform * Affine::translate((rect.x0, rect.y0));
                let (a1, a2, pie) = match arc {
                    Some(a) => (
                        (a.start_deg * 1_000_000.0).round() as i32,
                        (a.end_deg * 1_000_000.0).round() as i32,
                        u32::from(a.pie),
                    ),
                    None => (FULL_ELLIPSE_ANGLE, FULL_ELLIPSE_ANGLE, 0),
                };
                let mut c = Buf::default();
                c.i32(units(rect.width()))
                    .i32(units(rect.height()))
                    .i32(a1)
                    .i32(a2)
                    .u32(pie);
                self.place(s, OBJ_ELLIPSE, c.done(), &s.fill, t, page_box, None, centre)
            }
            // Text fitted to a path goes out as the outline of its letters
            // (below); other text as text.
            ShapeKind::Text {
                spans,
                origin,
                align,
                on_path: None,
                ..
            } => {
                let t = s.transform * Affine::translate(origin.to_vec2());
                self.has_text = true;
                let record = if v.0 >= 16 {
                    self.txsm16(
                        spans,
                        *align,
                        &s.fill,
                        Affine::translate(-centre.to_vec2()) * t,
                    )
                } else {
                    self.txsm7(spans, *align, &s.fill)
                };
                // The character settings of the first run, the width of
                // the lines as the frame's one column (files give artistic
                // text its line width there) and the alignment.
                let width = s.local_path().bounding_box().width();
                let character = format!(
                    r#"{},"frame":{{"equalWidthColumns":"1","rtlColumns":"0","columns":"{},0"}},"paragraph":{{"justify":"{}"}}"#,
                    spans
                        .first()
                        .map(|sp| character_json(sp, Some(&s.fill)))
                        .unwrap_or_else(|| r#""character":{}"#.to_string()),
                    units(if width.is_finite() { width } else { 0.0 }).max(0),
                    justify_code(*align)
                );
                let mut c = Buf::default();
                c.i32(0).i32(0);
                self.place(
                    s,
                    OBJ_ARTISTIC_TEXT,
                    c.done(),
                    &s.fill,
                    t,
                    page_box,
                    Some((record, character, style_for(*align))),
                    centre,
                )
            }
            _ => {
                let coords = match &s.kind {
                    ShapeKind::Path { path, .. } => curve_coords_typed(path, &s.node_types),
                    _ => curve_coords(&s.local_path()),
                };
                let Some(coords) = coords else {
                    return none();
                };
                self.place(
                    s,
                    OBJ_CURVE,
                    coords,
                    &s.fill,
                    s.transform,
                    page_box,
                    None,
                    centre,
                )
            }
        }
    }

    /// Paragraphs of the text: the runs between line breaks, each run with
    /// its own style.
    fn paragraphs(spans: &[TextSpan]) -> Vec<Vec<TextSpan>> {
        let mut out: Vec<Vec<TextSpan>> = vec![Vec::new()];
        for sp in spans {
            let mut parts = sp.text.split('\n').peekable();
            while let Some(part) = parts.next() {
                if !part.is_empty() {
                    let mut run = sp.clone();
                    run.text = part.replace('\r', "");
                    if let Some(last) = out.last_mut() {
                        last.push(run);
                    }
                }
                if parts.peek().is_some() {
                    out.push(Vec::new());
                }
            }
        }
        out
    }

    /// `txsm` of versions 7 to 15: one frame, a paragraph per line, one
    /// style per run (font, encoding, style flags, size and, when it
    /// differs from the object's, the fill), the text in UTF-16 from
    /// version 12 and in Windows-1252 before.
    fn txsm7(&mut self, spans: &[TextSpan], align: TextAlign, fill: &Fill) -> Vec<u8> {
        let v = self.v;
        let n = v.0;
        let mut t = Buf::default();
        t.u32(0).zeros(32); // artistic text (no frame flag)
        if n >= 15 {
            t.zeros(1);
        }
        if n < 8 {
            t.u32(0); // not on a path
        }
        t.u32(1).u32(1).zeros(48); // one frame, its id and matrix
        if n >= 8 {
            t.u32(0); // not on a path
            if n >= 15 {
                t.zeros(8);
            }
        }
        t.zeros(if n >= 15 {
            40
        } else if n >= 14 {
            36
        } else if n > 8 {
            34
        } else if n == 8 {
            32
        } else {
            36
        });
        let paragraphs = Self::paragraphs(spans);
        t.u32(paragraphs.len() as u32);
        for runs in &paragraphs {
            t.u32(style_for(align)).zeros(1);
            t.u32(runs.len() as u32);
            let mut text = String::new();
            for sp in runs {
                let font = self.font_id(&sp.font_family);
                let chars = sp.text.chars().count().min(65535) as u16;
                let span_fill = sp.fill.as_ref().filter(|f| *f != fill);
                let mut flags = 0x01u8 | 0x02 | 0x04;
                if span_fill.is_some() {
                    flags |= 0x40;
                }
                t.u16(chars).u8(flags);
                if n >= 8 {
                    t.u8(0);
                }
                t.u16(font).u16(0).u32(style_flags(sp));
                t.i32((sp.size_pt * 254000.0 / 72.0).round() as i32);
                if let Some(f) = span_fill {
                    let id = self.fill_id(f);
                    t.u32(id);
                    if v.tagged() {
                        t.zeros(48);
                    }
                }
                text.push_str(&sp.text);
            }
            let chars = text.chars().count();
            t.u32(chars as u32)
                .zeros(chars * if v.unicode() { 8 } else { 4 });
            if v.unicode() {
                let bytes = utf16(&text);
                t.u32(bytes.len() as u32).bytes(&bytes);
            } else {
                let bytes = cp1252(&text).unwrap_or_else(|| {
                    text.chars()
                        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
                        .collect()
                });
                t.bytes(&bytes);
            }
            t.u8(0); // no per-letter path data
        }
        t.done()
    }

    /// `txsm` of version 16 and later, as files of versions 17 to 21 store
    /// artistic text: the frame flag set, layout 1700 (1600 in version
    /// 16), one frame with the text's matrix, then a paragraph per line
    /// with the default style string (the first run's style and the
    /// alignment) and one record per run whose style string the letters
    /// select; text in UTF-16 (16) or one byte per letter (17 on:
    /// Windows-1252, or UTF-8 for letters it lacks).
    fn txsm16(&mut self, spans: &[TextSpan], align: TextAlign, fill: &Fill, m: Affine) -> Vec<u8> {
        let v = self.v;
        let mut t = Buf::default();
        t.u32(u32::MAX)
            .u32(u32::MAX)
            .u32(0)
            .u32(u32::MAX)
            .u32(8)
            .u32(13)
            .u32(0)
            .u32(3)
            .u32(0)
            .u16(if v.0 >= 17 { 1700 } else { 1600 })
            .zeros(3);
        // One frame: its id, the text's matrix, not on a path, 8 unknown.
        t.u32(1).u32(1);
        let c = m.as_coeffs();
        for x in [
            c[0],
            c[2],
            c[4] * UNITS_PER_MM,
            c[1],
            c[3],
            c[5] * UNITS_PER_MM,
        ] {
            t.f64(if x.is_finite() { x } else { 0.0 });
        }
        t.u32(0).zeros(8);
        let paragraphs = Self::paragraphs(spans);
        t.u32(paragraphs.len() as u32);
        for runs in &paragraphs {
            for sp in runs {
                self.font_id(&sp.font_family);
            }
            let first = runs.first().or(spans.first());
            let default = match first {
                Some(sp) => {
                    let json = run_style_json(sp, Some(fill));
                    // The alignment goes with the paragraph.
                    let mut j = json;
                    j.pop();
                    format!(
                        r#"{j},"paragraph":{{"justify":"{}"}}}}"#,
                        justify_code(align)
                    )
                }
                None => "{}".to_string(),
            };
            t.u32(style_for(align))
                .zeros(1)
                .u8(0)
                .bytes(&style_string(v, &default));
            // One record per run; a letter's override index selects it.
            t.u32(runs.len() as u32);
            for sp in runs {
                let json = run_style_json(sp, sp.fill.as_ref().or(Some(fill)));
                t.u16(0).u16(1).u16(0).bytes(&style_string(v, &json));
            }
            let mut text = String::new();
            let mut index = Vec::new();
            for (k, sp) in runs.iter().enumerate() {
                text.push_str(&sp.text);
                index.extend(std::iter::repeat_n(k as u8, sp.text.chars().count()));
            }
            t.u32(index.len() as u32);
            for k in &index {
                t.u16(0).u8(k.saturating_mul(2)).zeros(5);
            }
            let bytes = if v.byte_strings() {
                cp1252(&text).unwrap_or_else(|| text.as_bytes().to_vec())
            } else {
                utf16(&text)
            };
            t.u32(bytes.len() as u32).bytes(&bytes).u8(0);
        }
        t.done()
    }
}

#[cfg(test)]
mod tests;
