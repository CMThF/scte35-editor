use base64::Engine;
use scte35::encoding::CrcEncodable;
use scte35::parse_splice_info_section;
use scte35_editor::core::{OutputFormat, ParseSettings, Scte35Document};
use scte35_editor::io::InputFormat;

const SAMPLE_BASE64: &str = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";

#[test]
fn edit_pts_time() {
    let mut doc = Scte35Document::parse(
        SAMPLE_BASE64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed");
    doc.apply_sets(&[("splice_time.pts_time".to_string(), "180000".to_string())])
        .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    match parsed.splice_command {
        scte35::SpliceCommand::TimeSignal(time_signal) => {
            assert_eq!(time_signal.splice_time.pts_time, Some(180000));
        }
        _ => panic!("expected TimeSignal"),
    }
}

#[test]
fn edit_immediate() {
    let mut doc = Scte35Document::parse(
        SAMPLE_BASE64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed");
    doc.apply_sets(&[("splice_time.immediate".to_string(), "true".to_string())])
        .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    match parsed.splice_command {
        scte35::SpliceCommand::TimeSignal(time_signal) => {
            assert_eq!(time_signal.splice_time.pts_time, None);
            assert_eq!(time_signal.splice_time.time_specified_flag, 0);
        }
        _ => panic!("expected TimeSignal"),
    }
}

fn decode_base64(input: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(input.as_bytes())
        .expect("base64 decode failed")
}

#[test]
fn edit_splice_insert_fields() {
    let mut doc = new_splice_insert_doc();
    doc.apply_sets(&[
        (
            "splice_insert.splice_event_id".to_string(),
            "99".to_string(),
        ),
        (
            "splice_insert.out_of_network".to_string(),
            "false".to_string(),
        ),
        (
            "splice_insert.unique_program_id".to_string(),
            "7".to_string(),
        ),
        ("splice_insert.avail_num".to_string(), "2".to_string()),
        ("splice_insert.avails_expected".to_string(), "3".to_string()),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    match parsed.splice_command {
        scte35::SpliceCommand::SpliceInsert(insert) => {
            assert_eq!(insert.splice_event_id, 99u32 << 24);
            assert_eq!(insert.out_of_network_indicator, 0);
            assert_eq!(insert.unique_program_id, 7);
            assert_eq!(insert.avail_num, 2);
            assert_eq!(insert.avails_expected, 3);
        }
        _ => panic!("expected SpliceInsert"),
    }
}

#[test]
fn edit_splice_insert_duration() {
    let mut doc = new_splice_insert_doc();
    doc.apply_sets(&[("splice_insert.duration".to_string(), "90000".to_string())])
        .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    match parsed.splice_command {
        scte35::SpliceCommand::SpliceInsert(insert) => {
            let duration = insert.break_duration.expect("missing break_duration");
            assert_eq!(duration.duration, 90000);
            assert_eq!(insert.duration_flag, 1);
        }
        _ => panic!("expected SpliceInsert"),
    }
}

#[test]
fn edit_splice_insert_components() {
    let mut doc = new_splice_insert_doc();
    doc.apply_sets(&[
        (
            "splice_insert.program_splice".to_string(),
            "false".to_string(),
        ),
        (
            "splice_insert.component.add".to_string(),
            "tag=1,pts=270000".to_string(),
        ),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    match parsed.splice_command {
        scte35::SpliceCommand::SpliceInsert(insert) => {
            assert_eq!(insert.program_splice_flag, 0);
            assert_eq!(insert.components.len(), 1);
        }
        _ => panic!("expected SpliceInsert"),
    }
}

#[test]
fn edit_splice_schedule_fields() {
    let mut doc = new_splice_schedule_doc();
    doc.apply_sets(&[
        (
            "splice_schedule.splice_event_id".to_string(),
            "77".to_string(),
        ),
        (
            "splice_schedule.out_of_network".to_string(),
            "false".to_string(),
        ),
        (
            "splice_schedule.utc_splice_time".to_string(),
            "100".to_string(),
        ),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    match parsed.splice_command {
        scte35::SpliceCommand::SpliceSchedule(schedule) => {
            assert_eq!(schedule.splice_event_id, 0x01000000);
            assert_eq!(schedule.out_of_network_indicator, 0);
            assert_eq!(schedule.utc_splice_time, Some(0x017c0000));
            assert_eq!(schedule.splice_duration, None);
            assert_eq!(schedule.component_list.len(), 0);
        }
        _ => panic!("expected SpliceSchedule"),
    }
}

#[test]
fn edit_segmentation_descriptor() {
    let mut doc = new_time_signal_doc();
    doc.apply_sets(&[
        ("segmentation.event_id".to_string(), "5".to_string()),
        ("segmentation.type_id".to_string(), "48".to_string()),
        ("segmentation.upid_type".to_string(), "0".to_string()),
        ("segmentation.upid".to_string(), "0x".to_string()),
        ("segmentation.segment_num".to_string(), "1".to_string()),
        (
            "segmentation.segments_expected".to_string(),
            "1".to_string(),
        ),
        ("segmentation.sub_segment_num".to_string(), "0".to_string()),
        (
            "segmentation.sub_segments_expected".to_string(),
            "0".to_string(),
        ),
        ("segmentation[0].event_id".to_string(), "6".to_string()),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    assert!(!parsed.splice_descriptors.is_empty());
}

#[test]
fn edit_avail_descriptor() {
    let mut doc = new_time_signal_doc();
    doc.apply_sets(&[
        ("avail.provider_id".to_string(), "0x41424344".to_string()),
        ("avail.identifier".to_string(), "1129531753".to_string()),
        ("avail[0].provider_id".to_string(), "0x41424345".to_string()),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    assert!(!parsed.splice_descriptors.is_empty());
}

#[test]
fn edit_dtmf_descriptor() {
    let mut doc = new_time_signal_doc();
    doc.apply_sets(&[
        ("dtmf.preroll".to_string(), "10".to_string()),
        ("dtmf.chars".to_string(), "0x313233".to_string()),
        ("dtmf[0].preroll".to_string(), "11".to_string()),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    assert!(!parsed.splice_descriptors.is_empty());
}

#[test]
fn edit_time_descriptor() {
    let mut doc = new_time_signal_doc();
    doc.apply_sets(&[
        ("time.tai_seconds".to_string(), "0x000000000001".to_string()),
        ("time.tai_ns".to_string(), "0x00000001".to_string()),
        ("time.utc_offset".to_string(), "0x0001".to_string()),
        (
            "time[0].tai_seconds".to_string(),
            "0x000000000002".to_string(),
        ),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    assert!(!parsed.splice_descriptors.is_empty());
}

#[test]
fn edit_audio_descriptor() {
    let mut doc = new_time_signal_doc();
    doc.apply_sets(&[
        ("audio.components".to_string(), "0x1122".to_string()),
        ("audio[0].components".to_string(), "0x1123".to_string()),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    assert!(!parsed.splice_descriptors.is_empty());
}

#[test]
fn edit_unknown_descriptor() {
    let mut doc = new_time_signal_doc();
    doc.apply_sets(&[
        ("unknown.tag".to_string(), "7".to_string()),
        ("unknown.data".to_string(), "0x010203".to_string()),
        ("unknown[0].data".to_string(), "0x010204".to_string()),
    ])
    .expect("apply failed");
    let out = doc.render(OutputFormat::Base64).expect("render failed");
    let bytes = decode_base64(&out);
    let parsed = parse_splice_info_section(&bytes).expect("parse failed");
    assert!(!parsed.splice_descriptors.is_empty());
}

fn new_splice_insert_doc() -> Scte35Document {
    let insert = scte35::builders::SpliceInsertBuilder::new(1)
        .out_of_network(true)
        .at_pts(std::time::Duration::from_secs(5))
        .expect("splice insert builder failed")
        .build()
        .expect("splice insert build failed");
    let section = scte35::builders::SpliceInfoSectionBuilder::new()
        .splice_insert(insert)
        .build()
        .expect("section build failed");
    let base64 = base64::engine::general_purpose::STANDARD
        .encode(section.encode_with_crc().expect("encode failed"));
    Scte35Document::parse(
        &base64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed")
}

fn new_splice_schedule_doc() -> Scte35Document {
    let schedule = scte35::SpliceSchedule {
        splice_event_id: 1,
        splice_event_cancel_indicator: 0,
        reserved: 0x7F,
        out_of_network_indicator: 1,
        duration_flag: 0,
        splice_duration: None,
        utc_splice_time: Some(0),
        unique_program_id: 0,
        num_splice: 0,
        component_list: Vec::new(),
    };
    let section = scte35::builders::SpliceInfoSectionBuilder::new()
        .splice_command(scte35::SpliceCommand::SpliceSchedule(schedule))
        .build()
        .expect("section build failed");
    let base64 = base64::engine::general_purpose::STANDARD
        .encode(section.encode_with_crc().expect("encode failed"));
    Scte35Document::parse(
        &base64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed")
}

fn new_time_signal_doc() -> Scte35Document {
    let splice_time = scte35::builders::SpliceTimeBuilder::new()
        .at_pts(std::time::Duration::from_secs(10))
        .expect("splice time builder failed")
        .build()
        .expect("splice time build failed");
    let section = scte35::builders::SpliceInfoSectionBuilder::new()
        .time_signal(scte35::TimeSignal { splice_time })
        .build()
        .expect("section build failed");
    let base64 = base64::engine::general_purpose::STANDARD
        .encode(section.encode_with_crc().expect("encode failed"));
    Scte35Document::parse(
        &base64,
        InputFormat::Base64,
        ParseSettings {
            validate_crc: false,
        },
    )
    .expect("parse failed")
}
