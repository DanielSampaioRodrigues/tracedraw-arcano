use super::*;
use tracedraw_core::{
    document::{Layer, Page, ParagraphStyle},
    geometry::Size,
};

fn doc_with(shapes: Vec<Shape>) -> Document {
    let mut doc = Document::new("w", Size::new(200.0, 100.0));
    doc.pages[0].layers[0].name = "Ink".into();
    doc.pages[0].layers[0].shapes = shapes;
    doc
}

#[test]
fn rectangle_ellipse_curve_and_text_round_trip() {
    let mut doc = Document::new("w", Size::new(200.0, 100.0));
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 20.0, 60.0, 50.0),
            radius: 2.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::rgb8(255, 0, 0));
    let mut st = Stroke::new(Color::rgb8(0, 0, 255), 1.5);
    st.cap = LineCap::Round;
    st.join = LineJoin::Bevel;
    st.dash = vec![3.0, 1.0];
    st.scale_with_object = true;
    r.stroke = Some(st.clone());
    r.name = Some("box".into());
    r.opacity = 0.5;
    let mut e = Shape::new(
        ids.shape(),
        ShapeKind::Ellipse {
            rect: Rect::new(100.0, 10.0, 140.0, 40.0),
            arc: None,
        },
    );
    e.fill = Fill::Fountain(tracedraw_core::Fountain::two(
        FountainKind::Radial,
        Color::BLACK,
        Color::WHITE,
        0.0,
    ));
    e.stroke = None;
    let mut p = BezPath::new();
    p.move_to((10.0, 60.0));
    p.curve_to((20.0, 90.0), (40.0, 90.0), (50.0, 60.0));
    p.line_to((30.0, 55.0));
    p.close_path();
    let mut c = Shape::new(
        ids.shape(),
        ShapeKind::Path {
            path: p,
            closed: true,
        },
    );
    c.fill = Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0));
    c.stroke = Some(Stroke::hairline(Color::BLACK));
    c.transform = Affine::translate((100.0, 0.0));
    let mut t = Shape::new(
        ids.shape(),
        ShapeKind::Text {
            spans: vec![{
                let mut s = TextSpan::new("Hello", "Verdana", 24.0);
                s.bold = true;
                s
            }],
            origin: Point::new(20.0, 80.0),
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        },
    );
    t.fill = Fill::Solid(Color::rgb8(0, 128, 0));
    t.stroke = None;
    doc.pages[0].layers[0].name = "Ink".into();
    doc.pages[0].layers[0].shapes = vec![r, e, c, t];
    doc.set_ids(ids);
    for v in versions() {
        shapes_round_trip(&doc, v);
    }
}

fn shapes_round_trip(doc: &Document, v: u16) {
    let bytes = document_to_cdr_version(doc, v);
    let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
    assert_eq!(rep.version.map(|v| v.0), Some(v));
    assert_eq!(rep.skipped_objects, 0, "{v}: {:?}", rep.warnings);
    assert_eq!(back.pages.len(), 1);
    let page = &back.pages[0];
    assert!((page.size.width - 200.0).abs() < 0.01 && (page.size.height - 100.0).abs() < 0.01);
    assert_eq!(page.layers.len(), 1);
    assert_eq!(page.layers[0].name, "Ink");
    let shapes = &page.layers[0].shapes;
    assert_eq!(shapes.len(), 4, "{v}: {:?}", rep.warnings);
    // Order and geometry.
    let rb = shapes[0].bounds();
    assert!(
        (rb.x0 - 10.0).abs() < 0.01 && (rb.y1 - 50.0).abs() < 0.01,
        "{v}: {rb:?}"
    );
    if v < 15 {
        assert!(matches!(shapes[0].kind, ShapeKind::Rect { .. }), "{v}");
        if let ShapeKind::Rect { radius, .. } = shapes[0].kind {
            assert!((radius - 2.0).abs() < 0.01, "{v}");
        }
    } else {
        // Later rectangles are written as the curve they draw.
        assert!(
            matches!(shapes[0].kind, ShapeKind::Path { closed: true, .. }),
            "{v}"
        );
        let area = shapes[0].page_path().area().abs();
        let rounded = 50.0 * 30.0 - (4.0 - std::f64::consts::PI) * 4.0;
        assert!((area - rounded).abs() < 0.5, "{v}: {area}");
    }
    assert_eq!(shapes[0].fill, Fill::Solid(Color::rgb8(255, 0, 0)), "{v}");
    let s0 = shapes[0].stroke.as_ref().expect("stroke");
    assert_eq!(s0.color, Color::rgb8(0, 0, 255));
    assert!((s0.width - 1.5).abs() < 0.01);
    assert_eq!(s0.cap, LineCap::Round);
    assert_eq!(s0.join, LineJoin::Bevel);
    assert_eq!(s0.dash, vec![3.0, 1.0]);
    assert!(s0.scale_with_object, "{v}");
    assert_eq!(shapes[0].name.as_deref(), Some("box"), "{v}");
    assert!((shapes[0].opacity - 0.5).abs() < 0.01, "{v}");
    let eb = shapes[1].bounds();
    assert!(
        (eb.x0 - 100.0).abs() < 0.01 && (eb.y0 - 10.0).abs() < 0.01,
        "{eb:?}"
    );
    assert!(
        matches!(shapes[1].fill, Fill::Fountain(_)),
        "{v}: {:?}",
        shapes[1].fill
    );
    if let Fill::Fountain(f) = &shapes[1].fill {
        assert_eq!(f.kind, FountainKind::Radial, "{v}");
        assert_eq!(f.stops.len(), 2, "{v}");
        assert_eq!(f.stops[1].color, Color::WHITE, "{v}");
    }
    assert!(shapes[1].stroke.is_none());
    let cb = shapes[2].bounds();
    assert!(
        (cb.x0 - 110.0).abs() < 0.05 && (cb.x1 - 150.0).abs() < 0.05,
        "{cb:?}"
    );
    assert!(matches!(
        shapes[2].kind,
        ShapeKind::Path { closed: true, .. }
    ));
    assert_eq!(
        shapes[2].fill,
        Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0))
    );
    assert!((shapes[2].stroke.as_ref().expect("hairline").width - Stroke::HAIRLINE).abs() < 1e-9);
    match &shapes[3].kind {
        ShapeKind::Text { spans, .. } => {
            assert_eq!(spans.len(), 1, "{v}: {spans:?}");
            assert_eq!(spans[0].text, "Hello", "{v}");
            assert_eq!(spans[0].font_family, "Verdana", "{v}");
            assert!((spans[0].size_pt - 24.0).abs() < 0.01, "{v}");
            assert!(spans[0].bold, "{v}");
        }
        k => panic!("{v}: {k:?}"),
    }
    let o = shapes[3].transform * Point::ZERO;
    let origin = match &shapes[3].kind {
        ShapeKind::Text { origin, .. } => *origin,
        _ => unreachable!(),
    };
    let at = shapes[3].transform * origin;
    assert!(
        (at.x - 20.0).abs() < 0.05 && (at.y - 80.0).abs() < 0.05,
        "{v}: {o:?} {at:?}"
    );
    assert_eq!(shapes[3].fill, Fill::Solid(Color::rgb8(0, 128, 0)), "{v}");
}

#[test]
fn groups_bitmaps_and_several_pages_round_trip() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut a = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    a.fill = Fill::Solid(Color::BLACK);
    let mut b = a.clone();
    b.id = ids.shape();
    b.transform = Affine::translate((20.0, 0.0));
    let mut g = Shape::new(
        ids.shape(),
        ShapeKind::Group {
            children: vec![a, b],
        },
    );
    g.transform = Affine::translate((5.0, 5.0));
    let png = {
        let mut pm = tiny_skia::Pixmap::new(2, 2).expect("pixmap");
        let px = pm.pixels_mut();
        px[0] = tiny_skia::ColorU8::from_rgba(255, 0, 0, 255).premultiply();
        px[1] = tiny_skia::ColorU8::from_rgba(0, 255, 0, 255).premultiply();
        px[2] = tiny_skia::ColorU8::from_rgba(0, 0, 255, 255).premultiply();
        px[3] = tiny_skia::ColorU8::from_rgba(255, 255, 255, 255).premultiply();
        pm.encode_png().expect("png")
    };
    let mut bm = Shape::new(
        ids.shape(),
        ShapeKind::Bitmap {
            rect: Rect::new(100.0, 50.0, 140.0, 90.0),
            width_px: 2,
            height_px: 2,
            png,
            fx: None,
        },
    );
    bm.fill = Fill::None;
    bm.stroke = None;
    doc.pages[0].layers[0].shapes = vec![g, bm];
    let pid = ids.page();
    let lid = ids.layer();
    let mut second = Page {
        id: pid,
        name: "Back".into(),
        size: Size::new(200.0, 100.0),
        layers: vec![Layer::new(lid, "Second")],
        guides: Vec::new(),
        background: None,
    };
    let mut c = Shape::new(
        ids.shape(),
        ShapeKind::Ellipse {
            rect: Rect::new(10.0, 10.0, 30.0, 30.0),
            arc: None,
        },
    );
    c.fill = Fill::Solid(Color::rgb8(1, 2, 3));
    second.layers[0].shapes.push(c);
    doc.pages.push(second);
    doc.set_ids(ids);
    for v in versions() {
        groups_bitmaps_and_pages(&doc, v);
    }
}

fn groups_bitmaps_and_pages(doc: &Document, v: u16) {
    let bytes = document_to_cdr_version(doc, v);
    let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
    assert_eq!(back.pages.len(), 2, "{v}: {:?}", rep.warnings);
    assert_eq!(rep.skipped_objects, 0, "{:?}", rep.warnings);
    let l = &back.pages[0].layers[0];
    assert_eq!(l.shapes.len(), 2, "{v}: {:?}", rep.warnings);
    match &l.shapes[0].kind {
        ShapeKind::Group { children } => {
            assert_eq!(children.len(), 2);
            let gb = l.shapes[0].bounds();
            assert!(
                (gb.x0 - 5.0).abs() < 0.01 && (gb.x1 - 35.0).abs() < 0.01,
                "{gb:?}"
            );
        }
        k => panic!("{k:?}"),
    }
    match &l.shapes[1].kind {
        ShapeKind::Bitmap {
            width_px,
            height_px,
            png,
            ..
        } => {
            assert_eq!((*width_px, *height_px), (2, 2));
            let pm = tiny_skia::Pixmap::decode_png(png).expect("png");
            let p = pm.pixels();
            assert_eq!((p[0].red(), p[0].green(), p[0].blue()), (255, 0, 0));
            assert_eq!((p[3].red(), p[3].green(), p[3].blue()), (255, 255, 255));
        }
        k => panic!("{k:?}"),
    }
    let bb = l.shapes[1].bounds();
    assert!(
        (bb.x0 - 100.0).abs() < 0.01 && (bb.y1 - 90.0).abs() < 0.01,
        "{bb:?}"
    );
    assert_eq!(back.pages[1].layers[0].name, "Second");
    assert_eq!(back.pages[1].layers[0].shapes.len(), 1);
}

#[test]
fn a_clipped_bitmap_round_trips_as_a_crop_path() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let png = {
        let mut pm = tiny_skia::Pixmap::new(4, 4).expect("pixmap");
        pm.fill(tiny_skia::Color::from_rgba8(10, 20, 30, 255));
        pm.encode_png().expect("png")
    };
    let mut bm = Shape::new(
        ids.shape(),
        ShapeKind::Bitmap {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            width_px: 4,
            height_px: 4,
            png,
            fx: None,
        },
    );
    bm.fill = Fill::None;
    bm.stroke = None;
    bm.transform = Affine::translate((20.0, 20.0));
    let mut frame = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(40.0, 30.0, 80.0, 60.0),
            radius: 0.0,
            corners: None,
        },
    );
    frame.fill = Fill::None;
    frame.stroke = None;
    let mut pc = Shape::new(
        ids.shape(),
        ShapeKind::ClipFrame {
            frame: Box::new(frame),
            contents: vec![bm],
        },
    );
    pc.fill = Fill::None;
    pc.stroke = None;
    doc.pages[0].layers[0].shapes = vec![pc];
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        clipped_bitmap_reads_back(&back, &rep, v);
    }
}

fn clipped_bitmap_reads_back(back: &Document, rep: &crate::ParseReport, v: u16) {
    let l = &back.pages[0].layers[0];
    assert_eq!(l.shapes.len(), 1, "{v}: {:?}", rep.warnings);
    match &l.shapes[0].kind {
        ShapeKind::ClipFrame { frame, contents } => {
            // Frame and contents live in the clip frame's own space.
            let outer = l.shapes[0].transform;
            let fb = (outer * frame.page_path()).bounding_box();
            assert!(
                (fb.x0 - 40.0).abs() < 0.05 && (fb.x1 - 80.0).abs() < 0.05,
                "{fb:?}"
            );
            assert!(
                (fb.y0 - 30.0).abs() < 0.05 && (fb.y1 - 60.0).abs() < 0.05,
                "{fb:?}"
            );
            assert_eq!(contents.len(), 1);
            let cb = (outer * contents[0].page_path()).bounding_box();
            assert!(
                (cb.x0 - 20.0).abs() < 0.05 && (cb.x1 - 120.0).abs() < 0.05,
                "{cb:?}"
            );
        }
        k => panic!("{k:?}"),
    }
}

#[test]
fn written_files_survive_mutation() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::BLACK);
    doc.pages[0].layers[0].shapes = vec![r];
    doc.set_ids(ids);
    for v in [7, 12, 13, 15, 21] {
        let bytes = document_to_cdr_version(&doc, v);
        for cut in (0..bytes.len()).step_by(13) {
            let _ = crate::open_bytes(&bytes[..cut], "cut");
        }
        for i in (0..bytes.len()).step_by(7) {
            let mut m = bytes.clone();
            m[i] ^= 0x5a;
            let _ = crate::open_bytes(&m, "flip");
        }
    }
}

#[test]
fn corners_one_radius_cannot_hold_are_written_as_curves() {
    use tracedraw_core::{CornerKind, Corners};
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
    let mut per_corner = Corners::uniform(2.0, CornerKind::Round);
    per_corner.radii[Corners::TOP_LEFT] = 8.0;
    let mut fixed = Corners::uniform(3.0, CornerKind::Round);
    fixed.fixed = true;
    let mut shapes = Vec::new();
    for (corners, scale) in [
        (Corners::uniform(4.0, CornerKind::Scallop), (1.0, 1.0)),
        (per_corner, (1.0, 1.0)),
        (fixed, (2.0, 1.0)),
        (fixed, (2.0, 2.0)),
    ] {
        let mut s = Shape::new(ids.shape(), ShapeKind::rect_with_corners(rect, corners));
        s.transform = Affine::translate((20.0, 20.0)) * Affine::scale_non_uniform(scale.0, scale.1);
        s.fill = Fill::Solid(Color::BLACK);
        shapes.push(s);
    }
    doc.pages[0].layers[0].shapes = shapes.clone();
    doc.set_ids(ids);
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 4, "{v}: {:?}", rep.warnings);
        for (i, (a, b)) in shapes.iter().zip(read).enumerate() {
            let (aa, ab) = (a.page_path().area().abs(), b.page_path().area().abs());
            assert!((aa - ab).abs() < 0.5, "{v} {i}: {aa} vs {ab}");
            let (ba, bb) = (a.bounds(), b.bounds());
            assert!(
                (ba.x0 - bb.x0).abs() < 0.01 && (ba.y1 - bb.y1).abs() < 0.01,
                "{v} {i}"
            );
        }
        assert!(matches!(read[0].kind, ShapeKind::Path { closed: true, .. }));
        assert!(matches!(read[1].kind, ShapeKind::Path { closed: true, .. }));
        assert!(matches!(read[2].kind, ShapeKind::Path { closed: true, .. }));
        if v < 15 {
            // A fixed radius on an evenly scaled rectangle is still a
            // rectangle.
            assert!(
                matches!(read[3].kind, ShapeKind::Rect { radius, .. } if (radius - 1.5).abs() < 0.01),
                "{v}"
            );
        }
    }
}

#[test]
fn complex_stars_are_written_as_curves() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut star = Shape::new(
        ids.shape(),
        ShapeKind::Polygon {
            rect: Rect::new(10.0, 10.0, 50.0, 50.0),
            points: 6,
            sharpness: 0.0,
            complex: Some(1),
            twist: 0.0,
        },
    );
    star.fill = Fill::Solid(Color::BLACK);
    doc.pages[0].layers[0].shapes = vec![star.clone()];
    doc.set_ids(ids);
    for v in [7, 12, 16, 22] {
        complex_star_reads_back(&doc, &star, v);
    }
}

fn complex_star_reads_back(doc: &Document, star: &Shape, v: u16) {
    let (back, rep) =
        crate::open_bytes(&document_to_cdr_version(doc, v), "back").expect("reads back");
    let read = &back.pages[0].layers[0].shapes;
    assert_eq!(read.len(), 1, "{:?}", rep.warnings);
    let ShapeKind::Path { path, .. } = &read[0].kind else {
        panic!("{:?}", read[0].kind);
    };
    // Two triangles, as drawn.
    let moves = path
        .elements()
        .iter()
        .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
        .count();
    assert_eq!(moves, 2);
    let (a, b) = (
        star.page_path().area().abs(),
        read[0].page_path().area().abs(),
    );
    assert!((a - b).abs() < 0.5, "{a} vs {b}");
}

fn text_shape(
    ids: &mut tracedraw_core::id::IdSource,
    spans: Vec<TextSpan>,
    align: TextAlign,
) -> Shape {
    let mut t = Shape::new(
        ids.shape(),
        ShapeKind::Text {
            spans,
            origin: Point::new(0.0, 0.0),
            frame: None,
            align,
            para: tracedraw_core::document::ParagraphStyle::default(),
            on_path: None,
        },
    );
    t.transform = Affine::translate((50.0, 50.0));
    t.fill = Fill::Solid(Color::BLACK);
    t.stroke = None;
    t
}

/// Several runs and lines, each text with its alignment, read back the
/// same from every version.
#[test]
fn runs_lines_and_alignment_round_trip() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut red = TextSpan::new("Red ", "Georgia", 18.0);
    red.fill = Some(Fill::Solid(Color::rgb8(255, 0, 0)));
    red.italic = true;
    let mut big = TextSpan::new("big\nline two", "Verdana", 30.0);
    big.underline = true;
    let shapes = vec![
        text_shape(&mut ids, vec![red, big], TextAlign::Center),
        text_shape(
            &mut ids,
            vec![TextSpan::new("right", "Arial", 12.0)],
            TextAlign::Right,
        ),
    ];
    doc.pages[0].layers[0].shapes = shapes;
    doc.set_ids(ids);
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 2, "{v}: {:?}", rep.warnings);
        let ShapeKind::Text { spans, align, .. } = &read[0].kind else {
            panic!("{v}: {:?}", read[0].kind);
        };
        assert_eq!(*align, TextAlign::Center, "{v}");
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "Red big\nline two", "{v}: {spans:?}");
        let first = &spans[0];
        assert_eq!(first.font_family, "Georgia", "{v}");
        assert!(first.italic && !first.underline, "{v}");
        assert_eq!(first.fill, Some(Fill::Solid(Color::rgb8(255, 0, 0))), "{v}");
        let last = spans.last().expect("span");
        assert_eq!(last.font_family, "Verdana", "{v}");
        assert!((last.size_pt - 30.0).abs() < 0.01, "{v}");
        assert!(last.underline && !last.italic, "{v}");
        let ShapeKind::Text { align, .. } = &read[1].kind else {
            panic!("{v}: {:?}", read[1].kind);
        };
        assert_eq!(*align, TextAlign::Right, "{v}");
    }
}

/// Text fitted to a path goes out as the outline of its letters.
#[test]
fn text_on_a_path_is_written_as_its_outline() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut t = text_shape(
        &mut ids,
        vec![TextSpan::new("arc", "Arial", 12.0)],
        TextAlign::Left,
    );
    let mut arc = BezPath::new();
    arc.move_to((0.0, 0.0));
    arc.curve_to((10.0, 20.0), (30.0, 20.0), (40.0, 0.0));
    if let ShapeKind::Text { on_path, .. } = &mut t.kind {
        *on_path = Some(tracedraw_core::TextOnPath {
            path: arc,
            offset: 0.0,
            distance: 0.0,
            mirror: false,
        });
    }
    doc.pages[0].layers[0].shapes = vec![t];
    doc.set_ids(ids);
    for v in [7, 15, 21] {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 1, "{v}: {:?}", rep.warnings);
        assert!(
            matches!(read[0].kind, ShapeKind::Path { .. }),
            "{v}: {:?}",
            read[0].kind
        );
        assert_eq!(read[0].fill, Fill::Solid(Color::BLACK), "{v}");
    }
}

/// The chunks around one rectangle and one text, as files of the version
/// have them.
fn chunk_ids(bytes: &[u8]) -> (Vec<String>, crate::riff::Tree, Vec<u8>) {
    let c = crate::container::detect(bytes).expect("container");
    let riff = crate::container::riff_stream(bytes, &c).expect("riff");
    let ext = crate::container::external_streams(bytes, &c);
    let tree = crate::riff::parse_with_externals(&riff, ext, c.version.0).expect("tree");
    let mut ids = Vec::new();
    tree.root.walk(&mut |ch, depth| {
        let name = match ch.list_type_str() {
            Some(t) => format!("{}{}", "  ".repeat(depth), t),
            None => format!("{}{}", "  ".repeat(depth), ch.id_str()),
        };
        ids.push(name);
    });
    (ids, tree, riff)
}

#[test]
fn files_are_laid_out_as_the_versions_have_them() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::BLACK);
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Hi", "Arial", 12.0)],
        TextAlign::Left,
    );
    doc.pages[0].layers[0].shapes = vec![r, t];
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        // Containers: RIFF, then ZIP with the RIFF stream, then ZIP with
        // the chunk tree and data files.
        match v {
            7..=13 => assert_eq!(&bytes[..4], b"RIFF", "{v}"),
            _ => assert_eq!(&bytes[..4], b"PK\x03\x04", "{v}"),
        }
        if v >= 14 {
            let z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
            let names: Vec<&str> = z.file_names().collect();
            if v < 16 {
                assert_eq!(names, ["content/riffData.cdr"], "{v}");
            } else {
                for n in [
                    "content/root.dat",
                    "content/dataFileList.dat",
                    "content/data/data1.dat",
                    "content/data/masterPage.dat",
                    "content/data/page1.dat",
                ] {
                    assert!(names.contains(&n), "{v}: {names:?}");
                }
            }
        }
        let (ids, tree, riff) = chunk_ids(&bytes);
        let form = Ver(v).form();
        assert_eq!(tree.root.list_type, Some(form), "{v}");
        assert_eq!(&riff[8..12], &form, "{v}");
        let at = |name: &str| ids.iter().position(|i| i.trim() == name);
        // Version chunks, then the document tables, the master page and
        // the page.
        if v >= 16 {
            assert_eq!(ids[1].trim(), "fver", "{v}");
        }
        assert!(at("vrsn") < at("doc"), "{v}: {ids:?}");
        assert!(
            at("mcfg") < at("ptrt") && at("ptrt") < at("filt"),
            "{v}: {ids:?}"
        );
        assert!(
            at("filc").is_some() && at("otlt").is_some() && at("stlt").is_some(),
            "{v}"
        );
        let pages = ids.iter().filter(|i| i.trim() == "page").count();
        assert_eq!(pages, 2, "{v}: master page and page");
        // A page lists the master's guides and desktop layers, from 14 its
        // own guides, then the drawing's layer.
        let layers = ids.iter().filter(|i| i.trim() == "layr").count();
        assert_eq!(layers, if v >= 14 { 3 + 4 } else { 3 + 3 }, "{v}: {ids:?}");
        // Every object: flags, boxes, static id, attributes with the
        // transform; the text starts with its style and ends with its record.
        let obj = ids.iter().position(|i| i.trim() == "obj").expect("object");
        let next: Vec<&str> = ids[obj + 1..obj + 9].iter().map(|s| s.trim()).collect();
        assert_eq!(
            next,
            ["spnd", "flgs", "bbox", "obbx", "usdn", "lgob", "loda", "ftil"],
            "{v}: the text comes first, front to back"
        );
        assert!(ids.iter().any(|i| i.trim() == "txsm"), "{v}");
        let mut flags = Vec::new();
        tree.root.walk(&mut |ch, _| {
            if ch.is(b"flgs") {
                flags.push(tree.data(&riff, ch).to_vec());
            }
        });
        assert!(flags.contains(&vec![0, 2, 0, 9]), "{v}: text flags");
        assert!(flags.contains(&vec![0, 2, 0, 8]), "{v}: object flags");
        assert!(
            flags.iter().any(|f| f.get(2..4) == Some(&[1, 0x90])),
            "{v}: master page"
        );
    }
}

/// Fonts, style table, fill and outline records keep the sizes files of
/// the version have.
#[test]
fn records_have_the_sizes_of_the_versions_files() {
    let s = Stroke::new(Color::BLACK, 0.5);
    for (v, outline, solid, none) in [
        (11u16, 140usize, 36usize, 8usize),
        (14, 192, 76, 22),
        (15, 269, 97, 22),
        (17, 213, 97, 22),
        (21, 213, 97, 22),
        (22, 213, 97, 22),
    ] {
        assert_eq!(outl(Ver(v), 1, Some(&s)).len(), outline, "{v}");
        assert_eq!(
            fild(Ver(v), 1, &Fill::Solid(Color::BLACK)).len(),
            solid,
            "{v}"
        );
        assert_eq!(fild(Ver(v), 1, &Fill::None).len(), none, "{v}");
        assert_eq!(
            font_record(Ver(v), 1, "Arial").len(),
            if v >= 12 { 296 } else { 92 },
            "{v}"
        );
        assert_eq!(
            trfd(Ver(v), Affine::IDENTITY).len(),
            if v >= 13 { 96 } else { 80 }
        );
    }
    // Two-stop gradients: stops of 16 bytes before version 13, 24 in 13
    // and 14, 45 from 15.
    let f = Fill::Fountain(tracedraw_core::Fountain::two(
        FountainKind::Linear,
        Color::BLACK,
        Color::WHITE,
        0.0,
    ));
    let len = |v: u16| fild(Ver(v), 1, &f).len();
    assert_eq!(len(12), 56 + 2 * 16, "version 12");
    assert_eq!(
        len(15) - len(14),
        2 * (45 - 24),
        "a stop grows by 21 bytes in 15"
    );
}
