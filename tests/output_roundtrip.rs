use base64::Engine;
use scte35_editor::core::{OutputFormat, ParseSettings, Scte35Document};
use scte35_editor::io::InputFormat;

const SAMPLE_BASE64: &str = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";

#[test]
fn render_base64_roundtrip() {
    let doc = Scte35Document::parse(
        SAMPLE_BASE64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let original = parse_bytes(SAMPLE_BASE64);
    let rendered = parse_bytes(&out);
    assert_eq!(original, rendered);
}

#[test]
fn render_hex_roundtrip() {
    let doc = Scte35Document::parse(
        SAMPLE_BASE64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed");
    let out = doc.render(OutputFormat::Hex).expect("render failed");
    let original = parse_bytes(SAMPLE_BASE64);
    let rendered = hex::decode(out).expect("hex decode failed");
    assert_eq!(original, rendered);
}

#[test]
fn render_json_roundtrip() {
    let doc = Scte35Document::parse(
        SAMPLE_BASE64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed");
    let out = doc.render(OutputFormat::Json).expect("render failed");
    let parsed = Scte35Document::parse(
        &out,
        InputFormat::Json,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("json parse failed");
    let original = parse_bytes(SAMPLE_BASE64);
    let rendered = parse_bytes(&parsed.render(OutputFormat::Base64).expect("render failed"));
    assert_eq!(original, rendered);
}

fn parse_bytes(base64_input: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(base64_input.as_bytes())
        .expect("base64 decode failed")
}
