use base64::Engine;
use scte35::encoding::CrcEncodable;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

fn cargo_env() -> String {
    std::env::var("CARGO_HOME").unwrap_or_default()
}

fn binary_path() -> PathBuf {
    if let Ok(value) = std::env::var("CARGO_BIN_EXE_scte35-editor") {
        return PathBuf::from(value);
    }
    if let Ok(value) = std::env::var("CARGO_BIN_EXE_scte35_editor") {
        return PathBuf::from(value);
    }
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        return Path::new(&target_dir).join("debug").join("scte35-editor");
    }
    let exe = std::env::current_exe().expect("missing current exe");
    let deps_dir = exe.parent().expect("missing deps dir");
    let target_dir = deps_dir.parent().expect("missing target dir");
    target_dir.join("scte35-editor")
}

#[test]
fn show_outputs_json() {
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args(["show", "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A=="])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout).expect("json parse failed");
    assert!(value["json"].is_object());
    assert!(value["hex"].as_str().is_some());
    assert!(value["base64"].as_str().is_some());
}

#[test]
fn validate_success() {
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args(["validate", "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A=="])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_failure_bad_input() {
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args(["validate", "not-a-valid-message"])
        .output()
        .expect("failed to run binary");
    assert!(!output.status.success());
}

#[test]
fn delete_descriptor_by_index() {
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=",
            "--target",
            "segmentation[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\"descriptor_type\": \"Segmentation\""));
}

#[test]
fn delete_splice_insert_component_by_index() {
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==",
            "--target",
            "splice_insert.component[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("splice_insert"));
}

#[test]
fn delete_splice_insert_component_success() {
    let input = splice_insert_base64_with_component();
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            input.as_str(),
            "--target",
            "splice_insert.component[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn delete_avail_descriptor_by_index() {
    let input = time_signal_base64();
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "edit",
            input.as_str(),
            "--set",
            "avail.provider_id=0x41424344",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(output.status.success());
    let edited = String::from_utf8_lossy(&output.stdout);
    let edited = serde_json::from_str::<Value>(&edited).expect("json parse failed");
    let edited_json = serde_json::to_string(edited["json"].as_object().expect("json missing"))
        .expect("json serialize failed");

    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            edited_json.as_str(),
            "--input-format",
            "json",
            "--target",
            "avail[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn delete_dtmf_descriptor_by_index() {
    let input = time_signal_base64();
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "edit",
            input.as_str(),
            "--set",
            "dtmf.chars=0x313233",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(output.status.success());
    let edited = String::from_utf8_lossy(&output.stdout);
    let edited = serde_json::from_str::<Value>(&edited).expect("json parse failed");
    let edited_json = serde_json::to_string(edited["json"].as_object().expect("json missing"))
        .expect("json serialize failed");

    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            edited_json.as_str(),
            "--input-format",
            "json",
            "--target",
            "dtmf[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn delete_time_descriptor_by_index() {
    let input = time_signal_base64();
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "edit",
            input.as_str(),
            "--set",
            "time.tai_seconds=0x000000000001",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(output.status.success());
    let edited = String::from_utf8_lossy(&output.stdout);
    let edited = serde_json::from_str::<Value>(&edited).expect("json parse failed");
    let edited_json = serde_json::to_string(edited["json"].as_object().expect("json missing"))
        .expect("json serialize failed");

    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            edited_json.as_str(),
            "--input-format",
            "json",
            "--target",
            "time[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn delete_audio_descriptor_by_index() {
    let input = time_signal_base64();
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "edit",
            input.as_str(),
            "--set",
            "audio.components=0x1122",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(output.status.success());
    let edited = String::from_utf8_lossy(&output.stdout);
    let edited = serde_json::from_str::<Value>(&edited).expect("json parse failed");
    let edited_json = serde_json::to_string(edited["json"].as_object().expect("json missing"))
        .expect("json serialize failed");

    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            edited_json.as_str(),
            "--input-format",
            "json",
            "--target",
            "audio[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn delete_unknown_descriptor_by_index() {
    let input = time_signal_with_unknown_descriptor();
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            input.as_str(),
            "--target",
            "unknown[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(output.status.success());
}

#[test]
fn delete_splice_schedule_component_by_index() {
    let input = splice_schedule_json_with_component();
    let output = Command::new(binary_path())
        .env("CARGO_HOME", cargo_env())
        .args([
            "delete",
            input.as_str(),
            "--input-format",
            "json",
            "--target",
            "splice_schedule.component[0]",
            "--output-format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout).expect("json parse failed");
    let components = value["json"]["splice_command"]["component_list"]
        .as_array()
        .expect("component_list missing");
    assert!(components.is_empty());
}

fn splice_insert_base64_with_component() -> String {
    let insert = scte35::builders::SpliceInsertBuilder::new(1)
        .component_splice(vec![(1, Some(Duration::from_secs(5)))])
        .expect("splice insert builder failed")
        .build()
        .expect("splice insert build failed");
    let section = scte35::builders::SpliceInfoSectionBuilder::new()
        .splice_insert(insert)
        .build()
        .expect("section build failed");
    let bytes = section.encode_with_crc().expect("encode failed");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn time_signal_with_unknown_descriptor() -> String {
    let section = time_signal_with_descriptor(scte35::SpliceDescriptor::Unknown {
        tag: 0x99,
        length: 3,
        data: vec![0x01, 0x02, 0x03],
    });
    encode_section(section)
}

fn time_signal_with_descriptor(descriptor: scte35::SpliceDescriptor) -> scte35::SpliceInfoSection {
    let splice_time = scte35::builders::SpliceTimeBuilder::new()
        .at_pts(Duration::from_secs(10))
        .expect("splice time builder failed")
        .build()
        .expect("splice time build failed");
    scte35::builders::SpliceInfoSectionBuilder::new()
        .time_signal(scte35::TimeSignal { splice_time })
        .add_descriptor(descriptor)
        .build()
        .expect("section build failed")
}

fn encode_section(section: scte35::SpliceInfoSection) -> String {
    let bytes = section.encode_with_crc().expect("encode failed");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn time_signal_base64() -> String {
    let splice_time = scte35::builders::SpliceTimeBuilder::new()
        .at_pts(Duration::from_secs(10))
        .expect("splice time builder failed")
        .build()
        .expect("splice time build failed");
    let section = scte35::builders::SpliceInfoSectionBuilder::new()
        .time_signal(scte35::TimeSignal { splice_time })
        .build()
        .expect("section build failed");
    encode_section(section)
}

fn splice_schedule_json_with_component() -> String {
    let schedule = scte35::SpliceSchedule {
        splice_event_id: 1,
        splice_event_cancel_indicator: 0,
        reserved: 0x7F,
        out_of_network_indicator: 1,
        duration_flag: 0,
        splice_duration: None,
        utc_splice_time: Some(0),
        unique_program_id: 0,
        num_splice: 1,
        component_list: vec![scte35::ComponentSplice {
            component_tag: 1,
            reserved: 0,
            splice_mode_indicator: 0,
            duration_flag: 0,
            splice_duration: None,
            utc_splice_time: Some(10),
        }],
    };
    let section = scte35::builders::SpliceInfoSectionBuilder::new()
        .splice_command(scte35::SpliceCommand::SpliceSchedule(schedule))
        .build()
        .expect("section build failed");
    serde_json::to_string(&section).expect("json serialize failed")
}
