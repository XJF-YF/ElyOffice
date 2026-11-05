//! JSON replies preserve fractional coordinates and escaped titles.

use std::path::Path;

use ely_pdf_engine::{Doc, Worker, sample::one_page};

fn opened(path: &Path, bytes: &[u8]) -> (Worker, Doc) {
    std::fs::write(path, bytes).expect("writes");
    let mut worker =
        Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker");
    let doc = worker.open(path).expect("opens").doc;
    (worker, doc)
}

fn outline(values: &[f32], title: &str) -> Vec<u8> {
    let title: String = std::iter::once(0xfeff)
        .chain(title.encode_utf16())
        .map(|unit| format!("{unit:04X}"))
        .collect();
    let last = 5 + values.len();
    let child = last + 1;
    let mut objects = vec![format!(
        "<< /Type /Outlines /First 6 0 R /Last {last} 0 R /Count {} >>",
        values.len() + 1
    )];
    for (at, y) in values.iter().enumerate() {
        let id = at + 6;
        let prev = if at > 0 {
            format!(" /Prev {} 0 R", id - 1)
        } else {
            String::new()
        };
        let next = if id < last {
            format!(" /Next {} 0 R", id + 1)
        } else {
            String::new()
        };
        let down = if at == 0 {
            format!(" /First {child} 0 R /Last {child} 0 R /Count 1")
        } else {
            String::new()
        };
        objects.push(format!(
            "<< /Title <{title}> /Parent 5 0 R /Dest [3 0 R /XYZ 0 {} null]{prev}{next}{down} >>",
            792.0 - y
        ));
    }
    objects.push("<< /Title (Nested) /Parent 6 0 R /Dest [3 0 R /XYZ 0 791.75 null] >>".into());
    one_page(" /Outlines 5 0 R", "", "", &objects)
}

#[test]
fn bookmark_coordinates_round_trip_below_one_and_in_nested_entries() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let values = [0.5, 0.000_488_281_25, 0.0, 42.0];
    let (mut worker, doc) = opened(
        &dir.path().join("outline.pdf"),
        &outline(&values, "Heading"),
    );
    let marks = worker.outline(doc).expect("valid JSON bookmarks");
    assert_eq!(marks.iter().map(|mark| mark.y).collect::<Vec<_>>(), values);
    assert!(marks.iter().all(|mark| mark.page == Some(0)));
    assert_eq!(marks[0].down.len(), 1);
    assert_eq!(
        (marks[0].down[0].title.as_str(), marks[0].down[0].y),
        ("Nested", 0.25)
    );
}

#[test]
fn bookmark_titles_round_trip_control_characters_and_unicode() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let title = "书签🙂 \"quoted\" \\ slash \u{1}\u{b}";
    let (mut worker, doc) = opened(&dir.path().join("title.pdf"), &outline(&[42.0], title));
    let marks = worker.outline(doc).expect("valid JSON strings");
    assert_eq!(marks[0].title, title);
}

#[test]
fn link_rectangles_and_destinations_keep_fractional_values() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let link = "<< /Type /Annot /Subtype /Link /Rect [-0.5 791.25 10.5 792.5] /Dest [3 0 R /XYZ 0 791.5 null] >>".into();
    let bytes = one_page("", " /Annots [5 0 R]", "", &[link]);
    let (mut worker, doc) = opened(&dir.path().join("link.pdf"), &bytes);
    let links = worker.links(doc, 0).expect("valid JSON links");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].rect, [-0.5, -0.5, 10.5, 0.75]);
    assert_eq!((links[0].page, links[0].y), (Some(0), 0.5));
}

fn content() -> Vec<u8> {
    let objects = [
        "<< /Type /Annot /Subtype /Square /Rect [-0.5 780.75 10.5 792.5] /BS << /W 0 >> /C [1 0 0] /Contents (Square) >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Text) /V (Value) /Rect [-0.5 770.75 10.5 780.5] /P 3 0 R /DA (/F1 10 Tf 0 g) >>",
        "<< /Type /Annot /Subtype /Widget /FT /Sig /T (Signature) /Rect [-0.5 760.75 10.5 770.5] /P 3 0 R /F 4 >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 0 >>\nstream\n\nendstream",
    ].map(String::from);
    one_page(
        " /AcroForm << /Fields [6 0 R 7 0 R] /SigFlags 3 >>",
        " /Annots [5 0 R 6 0 R 7 0 R] /Resources << /Font << /F1 8 0 R >> /XObject << /Fm1 9 0 R >> >>",
        "BT /F1 0.5 Tf 1 0 0 1 -0.5 791.5 Tm (A) Tj ET\nq 1 0 0 1 -0.5 791.5 cm /Fm1 Do Q",
        &objects,
    )
}

#[test]
fn text_blocks_keep_subpoint_font_sizes_and_rectangles() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&dir.path().join("text.pdf"), &content());
    let blocks = worker.text_blocks(doc, 0).expect("valid JSON text blocks");
    let block = blocks
        .iter()
        .find(|block| block.text == "A")
        .expect("the page text");
    assert_eq!(block.size, 0.5);
    assert_eq!(block.rect[0], -0.5);
    assert!(
        block
            .rect
            .iter()
            .any(|value| *value != 0.0 && value.abs() < 1.0)
    );
}

#[test]
fn drawn_objects_keep_fractional_rectangles_in_buffer_json() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&dir.path().join("objects.pdf"), &content());
    let objects = worker.objects(doc, 0).expect("valid JSON objects");
    assert_eq!(objects.len(), 1);
    assert!(objects[0].form);
    assert_eq!(objects[0].rect, [-0.5, -0.5, 0.5, 0.5]);
}

#[test]
fn annotations_and_fields_keep_fractional_rectangles() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&dir.path().join("marks.pdf"), &content());
    let marks = worker
        .marks(doc, 0)
        .expect("valid JSON annotations and fields");
    let annot = marks
        .annots
        .iter()
        .find(|annot| annot.kind == "Square")
        .expect("square");
    assert_eq!(annot.rect, [-0.5, -0.5, 10.5, 11.25]);
    let field = marks
        .fields
        .iter()
        .find(|field| field.name == "Text")
        .expect("text field");
    assert_eq!(field.rect, [-0.5, 11.5, 10.5, 21.25]);
}

#[test]
fn empty_signature_fields_keep_fractional_rectangles() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("signature.pdf");
    let (mut worker, doc) = opened(&path, &content());
    let signatures = worker
        .signatures(doc, &path)
        .expect("valid JSON signatures");
    assert_eq!(signatures.len(), 1);
    assert_eq!(signatures[0].name, "Signature");
    assert!(signatures[0].signed.is_none());
    assert_eq!(signatures[0].rect, [-0.5, 21.5, 10.5, 31.25]);
}
