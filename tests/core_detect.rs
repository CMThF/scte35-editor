use base64::Engine;
use scte35::parse_splice_info_section;
use scte35_editor::core::{ParseSettings, Scte35Document};
use scte35_editor::io::InputFormat;

const SAMPLE_BASE64: &str = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";

#[test]
fn detects_json_input() {
    let json = r#"{"table_id":252,"section_syntax_indicator":0,"private_indicator":0,"sap_type":0,"section_length":22,"protocol_version":0,"encrypted_packet":0,"encryption_algorithm":0,"pts_adjustment":0,"cw_index":255,"tier":4095,"splice_command_length":5,"splice_command_type":6,"splice_command":{"type":"TimeSignal","splice_time":{"time_specified_flag":1,"pts_time":900000,"duration_info":{"ticks":900000,"seconds":10.0,"human_readable":"10.0s"}}},"descriptor_loop_length":0,"splice_descriptors":[],"alignment_stuffing_bits":"","e_crc_32":null,"crc_32":0}"#;
    let result = Scte35Document::parse(
        json,
        InputFormat::Auto,
        ParseSettings {
            validate_crc: false,
        },
    );
    assert!(result.is_ok());
}

#[test]
fn detects_hex_input() {
    let buffer = decode_base64(SAMPLE_BASE64);
    let hex = format!("0x{}", hex::encode(buffer));
    let result = Scte35Document::parse(
        &hex,
        InputFormat::Auto,
        ParseSettings {
            validate_crc: false,
        },
    );
    assert!(result.is_ok());
}

#[test]
fn detects_base64_input() {
    let result = Scte35Document::parse(
        SAMPLE_BASE64,
        InputFormat::Auto,
        ParseSettings {
            validate_crc: false,
        },
    );
    assert!(result.is_ok());
}

fn decode_base64(input: &str) -> Vec<u8> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.as_bytes())
        .expect("base64 decode failed");
    parse_splice_info_section(&bytes).expect("scte35 parse failed");
    bytes
}
