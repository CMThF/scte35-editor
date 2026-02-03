use crate::io::InputFormat;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use scte35::descriptors::{AudioDescriptor, AvailDescriptor, DtmfDescriptor, TimeDescriptor};
use scte35::encoding::CrcEncodable;
use scte35::{SpliceInfoSection, parse_splice_info_section};
use std::time::Duration;

#[derive(Copy, Clone, Debug)]
pub struct ParseSettings {
    pub validate_crc: bool,
}

#[derive(Debug)]
pub struct Scte35Document {
    section: SpliceInfoSection,
}

#[derive(Debug, Clone)]
pub struct Scte35Summary {
    pub table_id: u8,
    pub pts_adjustment: u64,
    pub tier: u16,
    pub cw_index: u8,
    pub splice_command: String,
    pub descriptor_count: usize,
}

#[derive(Copy, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Base64,
    Hex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchPath {
    TableId,
    PtsAdjustment,
    Tier,
    CwIndex,
    SpliceCommand,
    SpliceTimePtsTime,
    SpliceTimeImmediate,
    SpliceScheduleSpliceEventId,
    SpliceScheduleCancel,
    SpliceScheduleOutOfNetwork,
    SpliceScheduleUtcSpliceTime,
    SpliceScheduleUtcSpliceTimeClear,
    SpliceScheduleDuration,
    SpliceScheduleDurationClear,
    SpliceScheduleUniqueProgramId,
    SpliceScheduleComponentAdd,
    SpliceScheduleComponentClear,
    SpliceScheduleComponent {
        index: usize,
        field: SpliceScheduleComponentField,
    },
    SpliceInsertSpliceEventId,
    SpliceInsertCancel,
    SpliceInsertOutOfNetwork,
    SpliceInsertProgramSplice,
    SpliceInsertSpliceImmediate,
    SpliceInsertSpliceTimePtsTime,
    SpliceInsertSpliceTimeImmediate,
    SpliceInsertDuration,
    SpliceInsertDurationClear,
    SpliceInsertAutoReturn,
    SpliceInsertUniqueProgramId,
    SpliceInsertAvailNum,
    SpliceInsertAvailsExpected,
    SpliceInsertComponentAdd,
    SpliceInsertComponentClear,
    SpliceInsertComponent {
        index: usize,
        field: SpliceInsertComponentField,
    },
    SegmentationEventId,
    SegmentationCancel,
    SegmentationProgram,
    SegmentationDuration,
    SegmentationDurationClear,
    SegmentationDeliveryNotRestricted,
    SegmentationWebDeliveryAllowed,
    SegmentationNoRegionalBlackout,
    SegmentationArchiveAllowed,
    SegmentationDeviceRestrictions,
    SegmentationUpidType,
    SegmentationUpid,
    SegmentationTypeId,
    SegmentationSegmentNum,
    SegmentationSegmentsExpected,
    SegmentationSubSegmentNum,
    SegmentationSubSegmentsExpected,
    SegmentationSubSegmentClear,
    Descriptor {
        kind: DescriptorKind,
        index: usize,
        field: DescriptorField,
    },
    AvailProviderId,
    AvailIdentifier,
    DtmfPreroll,
    DtmfChars,
    DtmfIdentifier,
    TimeTaiSeconds,
    TimeTaiNs,
    TimeUtcOffset,
    TimeIdentifier,
    AudioComponents,
    AudioIdentifier,
    UnknownTag,
    UnknownData,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpliceInsertComponentField {
    Tag,
    PtsTime,
    Immediate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpliceScheduleComponentField {
    Tag,
    SpliceMode,
    Duration,
    UtcSpliceTime,
    DurationFlag,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DescriptorField {
    SegmentationEventId,
    SegmentationCancel,
    SegmentationProgram,
    SegmentationDuration,
    SegmentationDurationClear,
    SegmentationDeliveryNotRestricted,
    SegmentationWebDeliveryAllowed,
    SegmentationNoRegionalBlackout,
    SegmentationArchiveAllowed,
    SegmentationDeviceRestrictions,
    SegmentationUpidType,
    SegmentationUpid,
    SegmentationTypeId,
    SegmentationSegmentNum,
    SegmentationSegmentsExpected,
    SegmentationSubSegmentNum,
    SegmentationSubSegmentsExpected,
    SegmentationSubSegmentClear,
    AvailProviderId,
    AvailIdentifier,
    DtmfPreroll,
    DtmfChars,
    DtmfIdentifier,
    TimeTaiSeconds,
    TimeTaiNs,
    TimeUtcOffset,
    TimeIdentifier,
    AudioComponents,
    AudioIdentifier,
    UnknownTag,
    UnknownData,
}

#[derive(Clone, Debug)]
pub struct PatchOp {
    pub path: PatchPath,
    pub value: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PatchValueType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    Bytes,
    SpliceCommand,
    ComponentSpec,
}

#[derive(Copy, Clone, Debug)]
pub struct PatchPathMeta {
    pub path: &'static str,
    pub value_type: PatchValueType,
    pub constraints: &'static str,
    pub example: &'static str,
}

impl PatchOp {
    pub fn new(path: &str, value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        validate_patch_value(path, &value)?;
        Ok(Self {
            path: parse_patch_path(path)?,
            value,
        })
    }
}

pub fn supported_paths() -> &'static [&'static str] {
    static PATHS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    PATHS
        .get_or_init(|| {
            supported_paths_meta()
                .iter()
                .map(|meta| meta.path)
                .collect()
        })
        .as_slice()
}

pub fn supported_paths_meta() -> &'static [PatchPathMeta] {
    &[
        PatchPathMeta {
            path: "table_id",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "252",
        },
        PatchPathMeta {
            path: "pts_adjustment",
            value_type: PatchValueType::U64,
            constraints: "33-bit (masked)",
            example: "90000",
        },
        PatchPathMeta {
            path: "tier",
            value_type: PatchValueType::U16,
            constraints: "12-bit (masked)",
            example: "4095",
        },
        PatchPathMeta {
            path: "cw_index",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "255",
        },
        PatchPathMeta {
            path: "splice_command",
            value_type: PatchValueType::SpliceCommand,
            constraints: "splice_null|splice_insert|time_signal|splice_schedule|bandwidth_reservation|private_command",
            example: "time_signal",
        },
        PatchPathMeta {
            path: "splice_time.pts_time",
            value_type: PatchValueType::U64,
            constraints: "33-bit (masked)",
            example: "180000",
        },
        PatchPathMeta {
            path: "splice_time.immediate",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_schedule.splice_event_id",
            value_type: PatchValueType::U32,
            constraints: "0-2^32-1",
            example: "77",
        },
        PatchPathMeta {
            path: "splice_schedule.cancel",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "false",
        },
        PatchPathMeta {
            path: "splice_schedule.out_of_network",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_schedule.utc_splice_time",
            value_type: PatchValueType::U32,
            constraints: "seconds since epoch",
            example: "100",
        },
        PatchPathMeta {
            path: "splice_schedule.utc_splice_time.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears utc_splice_time",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_schedule.duration",
            value_type: PatchValueType::U32,
            constraints: "90kHz ticks",
            example: "90000",
        },
        PatchPathMeta {
            path: "splice_schedule.duration.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears duration",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_schedule.unique_program_id",
            value_type: PatchValueType::U16,
            constraints: "0-65535",
            example: "1",
        },
        PatchPathMeta {
            path: "splice_schedule.component.add",
            value_type: PatchValueType::ComponentSpec,
            constraints: "not supported by encoder",
            example: "tag=1,splice_mode=0,duration=120",
        },
        PatchPathMeta {
            path: "splice_schedule.component.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears component list",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_schedule.component[i].tag",
            value_type: PatchValueType::U8,
            constraints: "0-255 (existing component)",
            example: "1",
        },
        PatchPathMeta {
            path: "splice_schedule.component[i].splice_mode",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "0",
        },
        PatchPathMeta {
            path: "splice_schedule.component[i].duration",
            value_type: PatchValueType::U32,
            constraints: "90kHz ticks",
            example: "120",
        },
        PatchPathMeta {
            path: "splice_schedule.component[i].utc_splice_time",
            value_type: PatchValueType::U32,
            constraints: "seconds since epoch",
            example: "100",
        },
        PatchPathMeta {
            path: "splice_schedule.component[i].duration_flag",
            value_type: PatchValueType::Bool,
            constraints: "true=duration, false=utc",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_insert.splice_event_id",
            value_type: PatchValueType::U32,
            constraints: "0-2^32-1 (encoded in command)",
            example: "99",
        },
        PatchPathMeta {
            path: "splice_insert.cancel",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "false",
        },
        PatchPathMeta {
            path: "splice_insert.out_of_network",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_insert.program_splice",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_insert.splice_immediate",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "false",
        },
        PatchPathMeta {
            path: "splice_insert.splice_time.pts_time",
            value_type: PatchValueType::U64,
            constraints: "33-bit (masked)",
            example: "270000",
        },
        PatchPathMeta {
            path: "splice_insert.splice_time.immediate",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_insert.duration",
            value_type: PatchValueType::U64,
            constraints: "90kHz ticks",
            example: "90000",
        },
        PatchPathMeta {
            path: "splice_insert.duration.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears duration",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_insert.auto_return",
            value_type: PatchValueType::Bool,
            constraints: "requires duration",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_insert.unique_program_id",
            value_type: PatchValueType::U16,
            constraints: "0-65535",
            example: "1",
        },
        PatchPathMeta {
            path: "splice_insert.avail_num",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "1",
        },
        PatchPathMeta {
            path: "splice_insert.avails_expected",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "1",
        },
        PatchPathMeta {
            path: "splice_insert.component.add",
            value_type: PatchValueType::ComponentSpec,
            constraints: "tag=..., pts=... | immediate=true",
            example: "tag=1,pts=270000",
        },
        PatchPathMeta {
            path: "splice_insert.component.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears component list",
            example: "true",
        },
        PatchPathMeta {
            path: "splice_insert.component[i].tag",
            value_type: PatchValueType::U8,
            constraints: "0-255 (existing component)",
            example: "1",
        },
        PatchPathMeta {
            path: "splice_insert.component[i].pts_time",
            value_type: PatchValueType::U64,
            constraints: "33-bit (masked)",
            example: "270000",
        },
        PatchPathMeta {
            path: "splice_insert.component[i].immediate",
            value_type: PatchValueType::Bool,
            constraints: "true clears splice_time",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation.event_id",
            value_type: PatchValueType::U32,
            constraints: "0-2^32-1 (encoded)",
            example: "5",
        },
        PatchPathMeta {
            path: "segmentation.cancel",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "false",
        },
        PatchPathMeta {
            path: "segmentation.program",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation.duration",
            value_type: PatchValueType::U64,
            constraints: "90kHz ticks",
            example: "90000",
        },
        PatchPathMeta {
            path: "segmentation.duration.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears duration",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation.delivery_not_restricted",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation.web_delivery_allowed",
            value_type: PatchValueType::Bool,
            constraints: "only if delivery_not_restricted=false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation.no_regional_blackout",
            value_type: PatchValueType::Bool,
            constraints: "only if delivery_not_restricted=false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation.archive_allowed",
            value_type: PatchValueType::Bool,
            constraints: "only if delivery_not_restricted=false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation.device_restrictions",
            value_type: PatchValueType::U8,
            constraints: "only if delivery_not_restricted=false",
            example: "3",
        },
        PatchPathMeta {
            path: "segmentation.upid_type",
            value_type: PatchValueType::U8,
            constraints: "UPID type id",
            example: "0",
        },
        PatchPathMeta {
            path: "segmentation.upid",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x41424344",
        },
        PatchPathMeta {
            path: "segmentation.type_id",
            value_type: PatchValueType::U8,
            constraints: "segmentation_type_id",
            example: "48",
        },
        PatchPathMeta {
            path: "segmentation.segment_num",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "1",
        },
        PatchPathMeta {
            path: "segmentation.segments_expected",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "1",
        },
        PatchPathMeta {
            path: "segmentation.sub_segment_num",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "0",
        },
        PatchPathMeta {
            path: "segmentation.sub_segments_expected",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "0",
        },
        PatchPathMeta {
            path: "segmentation.sub_segment.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears sub-segment fields",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation[i].event_id",
            value_type: PatchValueType::U32,
            constraints: "0-2^32-1 (encoded)",
            example: "5",
        },
        PatchPathMeta {
            path: "segmentation[i].cancel",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "false",
        },
        PatchPathMeta {
            path: "segmentation[i].program",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation[i].duration",
            value_type: PatchValueType::U64,
            constraints: "90kHz ticks",
            example: "90000",
        },
        PatchPathMeta {
            path: "segmentation[i].duration.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears duration",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation[i].delivery_not_restricted",
            value_type: PatchValueType::Bool,
            constraints: "true|false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation[i].web_delivery_allowed",
            value_type: PatchValueType::Bool,
            constraints: "only if delivery_not_restricted=false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation[i].no_regional_blackout",
            value_type: PatchValueType::Bool,
            constraints: "only if delivery_not_restricted=false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation[i].archive_allowed",
            value_type: PatchValueType::Bool,
            constraints: "only if delivery_not_restricted=false",
            example: "true",
        },
        PatchPathMeta {
            path: "segmentation[i].device_restrictions",
            value_type: PatchValueType::U8,
            constraints: "only if delivery_not_restricted=false",
            example: "3",
        },
        PatchPathMeta {
            path: "segmentation[i].upid_type",
            value_type: PatchValueType::U8,
            constraints: "UPID type id",
            example: "0",
        },
        PatchPathMeta {
            path: "segmentation[i].upid",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x41424344",
        },
        PatchPathMeta {
            path: "segmentation[i].type_id",
            value_type: PatchValueType::U8,
            constraints: "segmentation_type_id",
            example: "48",
        },
        PatchPathMeta {
            path: "segmentation[i].segment_num",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "1",
        },
        PatchPathMeta {
            path: "segmentation[i].segments_expected",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "1",
        },
        PatchPathMeta {
            path: "segmentation[i].sub_segment_num",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "0",
        },
        PatchPathMeta {
            path: "segmentation[i].sub_segments_expected",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "0",
        },
        PatchPathMeta {
            path: "segmentation[i].sub_segment.clear",
            value_type: PatchValueType::Bool,
            constraints: "true clears sub-segment fields",
            example: "true",
        },
        PatchPathMeta {
            path: "avail.provider_id",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x41424344",
        },
        PatchPathMeta {
            path: "avail.identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "avail[i].provider_id",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x41424344",
        },
        PatchPathMeta {
            path: "avail[i].identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "dtmf.preroll",
            value_type: PatchValueType::U8,
            constraints: "90kHz ticks",
            example: "10",
        },
        PatchPathMeta {
            path: "dtmf.chars",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x313233",
        },
        PatchPathMeta {
            path: "dtmf.identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "dtmf[i].preroll",
            value_type: PatchValueType::U8,
            constraints: "90kHz ticks",
            example: "10",
        },
        PatchPathMeta {
            path: "dtmf[i].chars",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x313233",
        },
        PatchPathMeta {
            path: "dtmf[i].identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "time.tai_seconds",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x000000000001",
        },
        PatchPathMeta {
            path: "time.tai_ns",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x00000001",
        },
        PatchPathMeta {
            path: "time.utc_offset",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x0001",
        },
        PatchPathMeta {
            path: "time.identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "time[i].tai_seconds",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x000000000001",
        },
        PatchPathMeta {
            path: "time[i].tai_ns",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x00000001",
        },
        PatchPathMeta {
            path: "time[i].utc_offset",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x0001",
        },
        PatchPathMeta {
            path: "time[i].identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "audio.components",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x1122",
        },
        PatchPathMeta {
            path: "audio.identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "audio[i].components",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty",
            example: "0x1122",
        },
        PatchPathMeta {
            path: "audio[i].identifier",
            value_type: PatchValueType::U32,
            constraints: "identifier (default CUEI)",
            example: "1129531753",
        },
        PatchPathMeta {
            path: "unknown.tag",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "7",
        },
        PatchPathMeta {
            path: "unknown.data",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty; length auto-set",
            example: "0x010203",
        },
        PatchPathMeta {
            path: "unknown[i].tag",
            value_type: PatchValueType::U8,
            constraints: "0-255",
            example: "7",
        },
        PatchPathMeta {
            path: "unknown[i].data",
            value_type: PatchValueType::Bytes,
            constraints: "hex/base64/empty; length auto-set",
            example: "0x010203",
        },
    ]
}

pub fn patch_meta(path: &str) -> Option<&'static PatchPathMeta> {
    if let Some(meta) = supported_paths_meta().iter().find(|meta| meta.path == path) {
        return Some(meta);
    }
    if let Some(normalized) = normalize_indexed_path(path) {
        return supported_paths_meta()
            .iter()
            .find(|meta| meta.path == normalized);
    }
    None
}

pub fn validate_patch_value(path: &str, value: &str) -> Result<(), String> {
    let meta = patch_meta(path).ok_or_else(|| format!("unsupported path '{path}'"))?;
    if meta.path.contains("[i]") {
        match meta.value_type {
            PatchValueType::Bool => {
                parse_bool(path, value)?;
            }
            PatchValueType::U8 => {
                parse_u8(path, value)?;
            }
            PatchValueType::U16 => {
                parse_u16(path, value)?;
            }
            PatchValueType::U32 => {
                parse_u32(path, value)?;
            }
            PatchValueType::U64 => {
                parse_u64(path, value)?;
            }
            PatchValueType::Bytes => {
                parse_bytes(value)?;
            }
            PatchValueType::SpliceCommand => {}
            PatchValueType::ComponentSpec => {
                return Err(format!("unsupported component spec path '{path}'"));
            }
        }
        return Ok(());
    }
    match meta.value_type {
        PatchValueType::Bool => {
            parse_bool(path, value)?;
        }
        PatchValueType::U8 => {
            parse_u8(path, value)?;
        }
        PatchValueType::U16 => {
            parse_u16(path, value)?;
        }
        PatchValueType::U32 => {
            parse_u32(path, value)?;
        }
        PatchValueType::U64 => {
            parse_u64(path, value)?;
        }
        PatchValueType::Bytes => {
            parse_bytes(value)?;
        }
        PatchValueType::SpliceCommand => {
            let name = value.trim();
            match name {
                "splice_null"
                | "splice_insert"
                | "time_signal"
                | "splice_schedule"
                | "bandwidth_reservation"
                | "private_command" => {}
                _ => {
                    return Err(
                        "splice_command must be one of: splice_null, splice_insert, time_signal, splice_schedule, bandwidth_reservation, private_command".into()
                    );
                }
            }
        }
        PatchValueType::ComponentSpec => {
            if path == "splice_insert.component.add" {
                let _ = parse_splice_insert_component(value)?;
            } else if path == "splice_schedule.component.add" {
                return Err("splice_schedule components are not supported by encoder yet".into());
            } else {
                let normalized = normalize_indexed_path(path).unwrap_or(path);
                if normalized == "splice_insert.component[i].tag" {
                    parse_u8(path, value)?;
                } else if normalized == "splice_insert.component[i].pts_time" {
                    parse_u64(path, value)?;
                } else if normalized == "splice_insert.component[i].immediate" {
                    parse_bool(path, value)?;
                } else if normalized == "splice_schedule.component[i].tag" {
                    parse_u8(path, value)?;
                } else if normalized == "splice_schedule.component[i].splice_mode" {
                    parse_u8(path, value)?;
                } else if normalized == "splice_schedule.component[i].duration" {
                    parse_u32(path, value)?;
                } else if normalized == "splice_schedule.component[i].utc_splice_time" {
                    parse_u32(path, value)?;
                } else if normalized == "splice_schedule.component[i].duration_flag" {
                    parse_bool(path, value)?;
                } else {
                    return Err(format!("unsupported component spec path '{path}'"));
                }
            }
        }
    }
    Ok(())
}
pub fn parse_patch_path(path: &str) -> Result<PatchPath, String> {
    if let Some((index, field)) = parse_indexed(path, "splice_insert.component") {
        return Ok(PatchPath::SpliceInsertComponent {
            index,
            field: match field {
                "tag" => SpliceInsertComponentField::Tag,
                "pts_time" => SpliceInsertComponentField::PtsTime,
                "immediate" => SpliceInsertComponentField::Immediate,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    if let Some((index, field)) = parse_indexed(path, "splice_schedule.component") {
        return Ok(PatchPath::SpliceScheduleComponent {
            index,
            field: match field {
                "tag" => SpliceScheduleComponentField::Tag,
                "splice_mode" => SpliceScheduleComponentField::SpliceMode,
                "duration" => SpliceScheduleComponentField::Duration,
                "utc_splice_time" => SpliceScheduleComponentField::UtcSpliceTime,
                "duration_flag" => SpliceScheduleComponentField::DurationFlag,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    if let Some((index, field)) = parse_indexed(path, "segmentation") {
        return Ok(PatchPath::Descriptor {
            kind: DescriptorKind::Segmentation,
            index,
            field: match field {
                "event_id" => DescriptorField::SegmentationEventId,
                "cancel" => DescriptorField::SegmentationCancel,
                "program" => DescriptorField::SegmentationProgram,
                "duration" => DescriptorField::SegmentationDuration,
                "duration.clear" => DescriptorField::SegmentationDurationClear,
                "delivery_not_restricted" => DescriptorField::SegmentationDeliveryNotRestricted,
                "web_delivery_allowed" => DescriptorField::SegmentationWebDeliveryAllowed,
                "no_regional_blackout" => DescriptorField::SegmentationNoRegionalBlackout,
                "archive_allowed" => DescriptorField::SegmentationArchiveAllowed,
                "device_restrictions" => DescriptorField::SegmentationDeviceRestrictions,
                "upid_type" => DescriptorField::SegmentationUpidType,
                "upid" => DescriptorField::SegmentationUpid,
                "type_id" => DescriptorField::SegmentationTypeId,
                "segment_num" => DescriptorField::SegmentationSegmentNum,
                "segments_expected" => DescriptorField::SegmentationSegmentsExpected,
                "sub_segment_num" => DescriptorField::SegmentationSubSegmentNum,
                "sub_segments_expected" => DescriptorField::SegmentationSubSegmentsExpected,
                "sub_segment.clear" => DescriptorField::SegmentationSubSegmentClear,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    if let Some((index, field)) = parse_indexed(path, "avail") {
        return Ok(PatchPath::Descriptor {
            kind: DescriptorKind::Avail,
            index,
            field: match field {
                "provider_id" => DescriptorField::AvailProviderId,
                "identifier" => DescriptorField::AvailIdentifier,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    if let Some((index, field)) = parse_indexed(path, "dtmf") {
        return Ok(PatchPath::Descriptor {
            kind: DescriptorKind::Dtmf,
            index,
            field: match field {
                "preroll" => DescriptorField::DtmfPreroll,
                "chars" => DescriptorField::DtmfChars,
                "identifier" => DescriptorField::DtmfIdentifier,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    if let Some((index, field)) = parse_indexed(path, "time") {
        return Ok(PatchPath::Descriptor {
            kind: DescriptorKind::Time,
            index,
            field: match field {
                "tai_seconds" => DescriptorField::TimeTaiSeconds,
                "tai_ns" => DescriptorField::TimeTaiNs,
                "utc_offset" => DescriptorField::TimeUtcOffset,
                "identifier" => DescriptorField::TimeIdentifier,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    if let Some((index, field)) = parse_indexed(path, "audio") {
        return Ok(PatchPath::Descriptor {
            kind: DescriptorKind::Audio,
            index,
            field: match field {
                "components" => DescriptorField::AudioComponents,
                "identifier" => DescriptorField::AudioIdentifier,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    if let Some((index, field)) = parse_indexed(path, "unknown") {
        return Ok(PatchPath::Descriptor {
            kind: DescriptorKind::Unknown,
            index,
            field: match field {
                "tag" => DescriptorField::UnknownTag,
                "data" => DescriptorField::UnknownData,
                _ => return Err(format!("unsupported path '{path}'")),
            },
        });
    }
    match path {
        "table_id" => Ok(PatchPath::TableId),
        "pts_adjustment" => Ok(PatchPath::PtsAdjustment),
        "tier" => Ok(PatchPath::Tier),
        "cw_index" => Ok(PatchPath::CwIndex),
        "splice_command" => Ok(PatchPath::SpliceCommand),
        "splice_time.pts_time" => Ok(PatchPath::SpliceTimePtsTime),
        "splice_time.immediate" => Ok(PatchPath::SpliceTimeImmediate),
        "splice_schedule.splice_event_id" => Ok(PatchPath::SpliceScheduleSpliceEventId),
        "splice_schedule.cancel" => Ok(PatchPath::SpliceScheduleCancel),
        "splice_schedule.out_of_network" => Ok(PatchPath::SpliceScheduleOutOfNetwork),
        "splice_schedule.utc_splice_time" => Ok(PatchPath::SpliceScheduleUtcSpliceTime),
        "splice_schedule.utc_splice_time.clear" => Ok(PatchPath::SpliceScheduleUtcSpliceTimeClear),
        "splice_schedule.duration" => Ok(PatchPath::SpliceScheduleDuration),
        "splice_schedule.duration.clear" => Ok(PatchPath::SpliceScheduleDurationClear),
        "splice_schedule.unique_program_id" => Ok(PatchPath::SpliceScheduleUniqueProgramId),
        "splice_schedule.component.add" => Ok(PatchPath::SpliceScheduleComponentAdd),
        "splice_schedule.component.clear" => Ok(PatchPath::SpliceScheduleComponentClear),
        "splice_insert.splice_event_id" => Ok(PatchPath::SpliceInsertSpliceEventId),
        "splice_insert.cancel" => Ok(PatchPath::SpliceInsertCancel),
        "splice_insert.out_of_network" => Ok(PatchPath::SpliceInsertOutOfNetwork),
        "splice_insert.program_splice" => Ok(PatchPath::SpliceInsertProgramSplice),
        "splice_insert.splice_immediate" => Ok(PatchPath::SpliceInsertSpliceImmediate),
        "splice_insert.splice_time.pts_time" => Ok(PatchPath::SpliceInsertSpliceTimePtsTime),
        "splice_insert.splice_time.immediate" => Ok(PatchPath::SpliceInsertSpliceTimeImmediate),
        "splice_insert.duration" => Ok(PatchPath::SpliceInsertDuration),
        "splice_insert.duration.clear" => Ok(PatchPath::SpliceInsertDurationClear),
        "splice_insert.auto_return" => Ok(PatchPath::SpliceInsertAutoReturn),
        "splice_insert.unique_program_id" => Ok(PatchPath::SpliceInsertUniqueProgramId),
        "splice_insert.avail_num" => Ok(PatchPath::SpliceInsertAvailNum),
        "splice_insert.avails_expected" => Ok(PatchPath::SpliceInsertAvailsExpected),
        "splice_insert.component.add" => Ok(PatchPath::SpliceInsertComponentAdd),
        "splice_insert.component.clear" => Ok(PatchPath::SpliceInsertComponentClear),
        "segmentation.event_id" => Ok(PatchPath::SegmentationEventId),
        "segmentation.cancel" => Ok(PatchPath::SegmentationCancel),
        "segmentation.program" => Ok(PatchPath::SegmentationProgram),
        "segmentation.duration" => Ok(PatchPath::SegmentationDuration),
        "segmentation.duration.clear" => Ok(PatchPath::SegmentationDurationClear),
        "segmentation.delivery_not_restricted" => Ok(PatchPath::SegmentationDeliveryNotRestricted),
        "segmentation.web_delivery_allowed" => Ok(PatchPath::SegmentationWebDeliveryAllowed),
        "segmentation.no_regional_blackout" => Ok(PatchPath::SegmentationNoRegionalBlackout),
        "segmentation.archive_allowed" => Ok(PatchPath::SegmentationArchiveAllowed),
        "segmentation.device_restrictions" => Ok(PatchPath::SegmentationDeviceRestrictions),
        "segmentation.upid_type" => Ok(PatchPath::SegmentationUpidType),
        "segmentation.upid" => Ok(PatchPath::SegmentationUpid),
        "segmentation.type_id" => Ok(PatchPath::SegmentationTypeId),
        "segmentation.segment_num" => Ok(PatchPath::SegmentationSegmentNum),
        "segmentation.segments_expected" => Ok(PatchPath::SegmentationSegmentsExpected),
        "segmentation.sub_segment_num" => Ok(PatchPath::SegmentationSubSegmentNum),
        "segmentation.sub_segments_expected" => Ok(PatchPath::SegmentationSubSegmentsExpected),
        "segmentation.sub_segment.clear" => Ok(PatchPath::SegmentationSubSegmentClear),
        "avail.provider_id" => Ok(PatchPath::AvailProviderId),
        "avail.identifier" => Ok(PatchPath::AvailIdentifier),
        "dtmf.preroll" => Ok(PatchPath::DtmfPreroll),
        "dtmf.chars" => Ok(PatchPath::DtmfChars),
        "dtmf.identifier" => Ok(PatchPath::DtmfIdentifier),
        "time.tai_seconds" => Ok(PatchPath::TimeTaiSeconds),
        "time.tai_ns" => Ok(PatchPath::TimeTaiNs),
        "time.utc_offset" => Ok(PatchPath::TimeUtcOffset),
        "time.identifier" => Ok(PatchPath::TimeIdentifier),
        "audio.components" => Ok(PatchPath::AudioComponents),
        "audio.identifier" => Ok(PatchPath::AudioIdentifier),
        "unknown.tag" => Ok(PatchPath::UnknownTag),
        "unknown.data" => Ok(PatchPath::UnknownData),
        _ => Err(format!("unsupported path '{path}'")),
    }
}

fn parse_indexed<'a>(path: &'a str, prefix: &str) -> Option<(usize, &'a str)> {
    let rest = path.strip_prefix(prefix)?;
    let rest = rest.strip_prefix('[')?;
    let mut parts = rest.splitn(2, ']');
    let index_str = parts.next()?;
    let index = index_str.parse::<usize>().ok()?;
    let remainder = parts.next()?;
    let remainder = remainder.strip_prefix('.').unwrap_or("");
    if remainder.is_empty() {
        return None;
    }
    Some((index, remainder))
}

fn parse_index_only(path: &str, prefix: &str) -> Option<usize> {
    let rest = path.strip_prefix(prefix)?;
    let rest = rest.strip_prefix('[')?;
    let mut parts = rest.splitn(2, ']');
    let index_str = parts.next()?;
    let index = index_str.parse::<usize>().ok()?;
    let remainder = parts.next().unwrap_or("");
    if remainder.is_empty() {
        Some(index)
    } else {
        None
    }
}

fn ensure_descriptor_index(
    section: &mut SpliceInfoSection,
    kind: DescriptorKind,
    index: usize,
) -> Result<(), String> {
    let mut count = section
        .splice_descriptors
        .iter()
        .filter(|d| match (d, &kind) {
            (scte35::SpliceDescriptor::Segmentation(_), DescriptorKind::Segmentation) => true,
            (scte35::SpliceDescriptor::Avail(_), DescriptorKind::Avail) => true,
            (scte35::SpliceDescriptor::Dtmf(_), DescriptorKind::Dtmf) => true,
            (scte35::SpliceDescriptor::Time(_), DescriptorKind::Time) => true,
            (scte35::SpliceDescriptor::Audio(_), DescriptorKind::Audio) => true,
            (scte35::SpliceDescriptor::Unknown { .. }, DescriptorKind::Unknown) => true,
            _ => false,
        })
        .count();

    while count <= index {
        let descriptor = match kind {
            DescriptorKind::Segmentation => {
                scte35::SpliceDescriptor::Segmentation(scte35::SegmentationDescriptor {
                    segmentation_event_id: 1,
                    segmentation_event_cancel_indicator: false,
                    program_segmentation_flag: true,
                    segmentation_duration_flag: false,
                    delivery_not_restricted_flag: true,
                    web_delivery_allowed_flag: None,
                    no_regional_blackout_flag: None,
                    archive_allowed_flag: None,
                    device_restrictions: None,
                    segmentation_duration: None,
                    segmentation_upid_type: scte35::SegmentationUpidType::NotUsed,
                    segmentation_upid_length: 0,
                    segmentation_upid: Vec::new(),
                    segmentation_type_id: 0,
                    segmentation_type: scte35::SegmentationType::NotIndicated,
                    segment_num: 0,
                    segments_expected: 0,
                    sub_segment_num: None,
                    sub_segments_expected: None,
                })
            }
            DescriptorKind::Avail => scte35::SpliceDescriptor::Avail(AvailDescriptor {
                identifier: 0x4355_4549,
                provider_avail_id: Vec::new(),
            }),
            DescriptorKind::Dtmf => scte35::SpliceDescriptor::Dtmf(DtmfDescriptor {
                identifier: 0x4355_4549,
                preroll: 0,
                dtmf_count: 0,
                dtmf_chars: Vec::new(),
            }),
            DescriptorKind::Time => scte35::SpliceDescriptor::Time(TimeDescriptor {
                identifier: 0x4355_4549,
                tai_seconds: Vec::new(),
                tai_ns: Vec::new(),
                utc_offset: Vec::new(),
            }),
            DescriptorKind::Audio => scte35::SpliceDescriptor::Audio(AudioDescriptor {
                identifier: 0x4355_4549,
                audio_components: Vec::new(),
            }),
            DescriptorKind::Unknown => scte35::SpliceDescriptor::Unknown {
                tag: 0,
                length: 0,
                data: Vec::new(),
            },
        };
        section.splice_descriptors.push(descriptor);
        count += 1;
    }

    Ok(())
}

fn normalize_indexed_path(path: &str) -> Option<&'static str> {
    for meta in supported_paths_meta() {
        if meta.path.contains("[i]") {
            let prefix = meta.path.split("[i]").next().unwrap_or("");
            if path.starts_with(prefix) {
                let rest = &path[prefix.len()..];
                if rest.starts_with('[') && rest.contains(']') {
                    let suffix = meta.path.split("[i]").nth(1).unwrap_or("");
                    if rest.ends_with(suffix) {
                        return Some(meta.path);
                    }
                }
            }
        }
    }
    None
}
impl Scte35Document {
    pub(crate) fn section(&self) -> &SpliceInfoSection {
        &self.section
    }

    pub fn summary(&self) -> Scte35Summary {
        let splice_command = match &self.section.splice_command {
            scte35::SpliceCommand::SpliceNull => "splice_null",
            scte35::SpliceCommand::SpliceInsert(_) => "splice_insert",
            scte35::SpliceCommand::TimeSignal(_) => "time_signal",
            scte35::SpliceCommand::SpliceSchedule(_) => "splice_schedule",
            scte35::SpliceCommand::BandwidthReservation(_) => "bandwidth_reservation",
            scte35::SpliceCommand::PrivateCommand(_) => "private_command",
            scte35::SpliceCommand::Unknown => "unknown",
        }
        .to_string();
        Scte35Summary {
            table_id: self.section.table_id,
            pts_adjustment: self.section.pts_adjustment,
            tier: self.section.tier,
            cw_index: self.section.cw_index,
            splice_command,
            descriptor_count: self.section.splice_descriptors.len(),
        }
    }
    pub fn parse(
        input: &str,
        format: InputFormat,
        settings: ParseSettings,
    ) -> Result<Self, String> {
        let format = match format {
            InputFormat::Auto => detect_format(input)?,
            other => other,
        };
        match format {
            InputFormat::Json => {
                let section = serde_json::from_str::<SpliceInfoSection>(input)
                    .map_err(|err| format!("json parse error: {err}"))?;
                Ok(Self { section })
            }
            InputFormat::Base64 => {
                let bytes = BASE64_STANDARD
                    .decode(input.trim())
                    .map_err(|err| format!("base64 decode error: {err}"))?;
                parse_message_bytes(&bytes, settings)
            }
            InputFormat::Hex => {
                let sanitized = input.trim().strip_prefix("0x").unwrap_or(input).trim();
                let bytes =
                    hex::decode(sanitized).map_err(|err| format!("hex decode error: {err}"))?;
                parse_message_bytes(&bytes, settings)
            }
            InputFormat::Auto => Err("input format could not be detected".into()),
        }
    }

    pub fn render(&self, format: OutputFormat) -> Result<String, String> {
        match format {
            OutputFormat::Json => serde_json::to_string_pretty(&self.section)
                .map_err(|err| format!("json serialize error: {err}")),
            OutputFormat::Base64 => {
                let bytes = self
                    .section
                    .encode_with_crc()
                    .map_err(|err| format!("encode error: {err}"))?;
                Ok(BASE64_STANDARD.encode(bytes))
            }
            OutputFormat::Hex => {
                let bytes = self
                    .section
                    .encode_with_crc()
                    .map_err(|err| format!("encode error: {err}"))?;
                Ok(hex::encode(bytes))
            }
        }
    }

    pub fn new_default_time_signal() -> Result<Self, String> {
        let splice_time = scte35::builders::SpliceTimeBuilder::new()
            .at_pts(Duration::from_secs(10))
            .map_err(|err| format!("splice time error: {err}"))?
            .build()
            .map_err(|err| format!("splice time error: {err}"))?;

        let time_signal = scte35::TimeSignal { splice_time };
        let section = scte35::builders::SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .build()
            .map_err(|err| format!("build error: {err}"))?;

        Ok(Scte35Document { section })
    }

    pub fn new_default_splice_null() -> Result<Self, String> {
        let section = scte35::builders::SpliceInfoSectionBuilder::new()
            .splice_null()
            .build()
            .map_err(|err| format!("build error: {err}"))?;
        Ok(Scte35Document { section })
    }

    pub fn new_default_splice_insert() -> Result<Self, String> {
        let insert = scte35::builders::SpliceInsertBuilder::new(1)
            .out_of_network(true)
            .at_pts(Duration::from_secs(10))
            .map_err(|err| format!("splice_insert builder error: {err}"))?
            .build()
            .map_err(|err| format!("splice_insert build error: {err}"))?;
        let section = scte35::builders::SpliceInfoSectionBuilder::new()
            .splice_insert(insert)
            .build()
            .map_err(|err| format!("build error: {err}"))?;
        Ok(Scte35Document { section })
    }

    pub fn new_default_splice_schedule() -> Result<Self, String> {
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
            .map_err(|err| format!("build error: {err}"))?;
        Ok(Scte35Document { section })
    }

    pub fn apply_sets(&mut self, sets: &[(String, String)]) -> Result<(), String> {
        let ops = sets
            .iter()
            .map(|(path, value)| PatchOp::new(path, value.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        self.apply_ops(&ops)
    }

    pub fn apply_ops(&mut self, ops: &[PatchOp]) -> Result<(), String> {
        for op in ops {
            self.apply_op(op)?;
        }
        Ok(())
    }

    pub fn delete_target(&mut self, target: &str) -> Result<(), String> {
        if let Some(index) = parse_index_only(target, "splice_insert.component") {
            let insert = splice_insert_mut(&mut self.section)?;
            if index >= insert.components.len() {
                return Err(format!("splice_insert.component[{index}] not found"));
            }
            insert.components.remove(index);
            insert.component_count = insert.components.len() as u8;
            return Ok(());
        }
        if let Some(index) = parse_index_only(target, "splice_schedule.component") {
            let schedule = splice_schedule_mut(&mut self.section)?;
            if index >= schedule.component_list.len() {
                return Err(format!("splice_schedule.component[{index}] not found"));
            }
            schedule.component_list.remove(index);
            schedule.num_splice = schedule.component_list.len() as u8;
            return Ok(());
        }
        if let Some(index) = parse_index_only(target, "segmentation") {
            return remove_descriptor_at(&mut self.section, DescriptorKind::Segmentation, index);
        }
        if let Some(index) = parse_index_only(target, "avail") {
            return remove_descriptor_at(&mut self.section, DescriptorKind::Avail, index);
        }
        if let Some(index) = parse_index_only(target, "dtmf") {
            return remove_descriptor_at(&mut self.section, DescriptorKind::Dtmf, index);
        }
        if let Some(index) = parse_index_only(target, "time") {
            return remove_descriptor_at(&mut self.section, DescriptorKind::Time, index);
        }
        if let Some(index) = parse_index_only(target, "audio") {
            return remove_descriptor_at(&mut self.section, DescriptorKind::Audio, index);
        }
        if let Some(index) = parse_index_only(target, "unknown") {
            return remove_descriptor_at(&mut self.section, DescriptorKind::Unknown, index);
        }
        Err(format!("unsupported delete target '{target}'"))
    }

    fn apply_op(&mut self, op: &PatchOp) -> Result<(), String> {
        let value = op.value.as_str();
        match &op.path {
            PatchPath::TableId => {
                self.section.table_id = parse_u8("table_id", value)?;
            }
            PatchPath::PtsAdjustment => {
                self.section.pts_adjustment = parse_u64("pts_adjustment", value)? & 0x1_FFFF_FFFF;
            }
            PatchPath::Tier => {
                self.section.tier = parse_u16("tier", value)? & 0x0FFF;
            }
            PatchPath::CwIndex => {
                self.section.cw_index = parse_u8("cw_index", value)?;
            }
            PatchPath::SpliceCommand => {
                let name = value.trim();
                self.section.splice_command = match name {
                    "splice_null" => scte35::SpliceCommand::SpliceNull,
                    "splice_insert" => {
                        let insert = scte35::builders::SpliceInsertBuilder::new(1)
                            .at_pts(Duration::from_secs(10))
                            .map_err(|err| format!("splice_insert builder error: {err}"))?
                            .build()
                            .map_err(|err| format!("splice_insert build error: {err}"))?;
                        scte35::SpliceCommand::SpliceInsert(insert)
                    }
                    "time_signal" => {
                        let splice_time = scte35::builders::SpliceTimeBuilder::new()
                            .at_pts(Duration::from_secs(10))
                            .map_err(|err| format!("splice_time error: {err}"))?
                            .build()
                            .map_err(|err| format!("splice_time error: {err}"))?;
                        scte35::SpliceCommand::TimeSignal(scte35::TimeSignal { splice_time })
                    }
                    "splice_schedule" => {
                        scte35::SpliceCommand::SpliceSchedule(scte35::SpliceSchedule {
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
                        })
                    }
                    "bandwidth_reservation" => {
                        scte35::SpliceCommand::BandwidthReservation(scte35::BandwidthReservation {
                            reserved: 0,
                            dwbw_reservation: 0,
                        })
                    }
                    "private_command" => {
                        scte35::SpliceCommand::PrivateCommand(scte35::PrivateCommand {
                            private_command_id: 0,
                            private_command_length: 0,
                            private_bytes: Vec::new(),
                        })
                    }
                    other => return Err(format!("unsupported splice_command '{other}'")),
                };
            }
            PatchPath::SpliceTimePtsTime => {
                let pts = parse_u64("splice_time.pts_time", value)? & 0x1_FFFF_FFFF;
                let splice_time = time_signal_mut(&mut self.section)?;
                splice_time.time_specified_flag = 1;
                splice_time.pts_time = Some(pts);
            }
            PatchPath::SpliceTimeImmediate => {
                let immediate = parse_bool("splice_time.immediate", value)?;
                let splice_time = time_signal_mut(&mut self.section)?;
                if immediate {
                    splice_time.time_specified_flag = 0;
                    splice_time.pts_time = None;
                } else if splice_time.pts_time.is_none() {
                    return Err("splice_time.immediate=false requires splice_time.pts_time".into());
                }
            }
            PatchPath::SpliceScheduleSpliceEventId => {
                let schedule = splice_schedule_mut(&mut self.section)?;
                schedule.splice_event_id = parse_u32("splice_schedule.splice_event_id", value)?;
                schedule.reserved = 0;
            }
            PatchPath::SpliceScheduleCancel => {
                splice_schedule_mut(&mut self.section)?.splice_event_cancel_indicator =
                    parse_bool("splice_schedule.cancel", value)? as u8;
            }
            PatchPath::SpliceScheduleOutOfNetwork => {
                splice_schedule_mut(&mut self.section)?.out_of_network_indicator =
                    parse_bool("splice_schedule.out_of_network", value)? as u8;
            }
            PatchPath::SpliceScheduleUtcSpliceTime => {
                let utc = parse_u32("splice_schedule.utc_splice_time", value)?;
                let schedule = splice_schedule_mut(&mut self.section)?;
                schedule.duration_flag = 0;
                schedule.splice_duration = None;
                schedule.utc_splice_time = Some(utc);
            }
            PatchPath::SpliceScheduleUtcSpliceTimeClear => {
                let clear = parse_bool("splice_schedule.utc_splice_time.clear", value)?;
                if clear {
                    splice_schedule_mut(&mut self.section)?.utc_splice_time = None;
                }
            }
            PatchPath::SpliceScheduleDuration => {
                let duration = parse_u32("splice_schedule.duration", value)?;
                let schedule = splice_schedule_mut(&mut self.section)?;
                schedule.duration_flag = 1;
                schedule.splice_duration = Some(duration);
                schedule.utc_splice_time = None;
            }
            PatchPath::SpliceScheduleDurationClear => {
                let clear = parse_bool("splice_schedule.duration.clear", value)?;
                if clear {
                    let schedule = splice_schedule_mut(&mut self.section)?;
                    schedule.duration_flag = 0;
                    schedule.splice_duration = None;
                }
            }
            PatchPath::SpliceScheduleUniqueProgramId => {
                splice_schedule_mut(&mut self.section)?.unique_program_id =
                    parse_u16("splice_schedule.unique_program_id", value)?;
            }
            PatchPath::SpliceScheduleComponentAdd => {
                return Err("splice_schedule components are not supported by encoder yet".into());
            }
            PatchPath::SpliceScheduleComponentClear => {
                let clear = parse_bool("splice_schedule.component.clear", value)?;
                if clear {
                    let schedule = splice_schedule_mut(&mut self.section)?;
                    schedule.component_list.clear();
                    schedule.num_splice = 0;
                }
            }
            PatchPath::SpliceInsertOutOfNetwork => {
                let out = parse_bool("splice_insert.out_of_network", value)?;
                splice_insert_mut(&mut self.section)?.out_of_network_indicator = out as u8;
            }
            PatchPath::SpliceInsertSpliceImmediate => {
                let immediate = parse_bool("splice_insert.splice_immediate", value)?;
                let insert = splice_insert_mut(&mut self.section)?;
                insert.splice_immediate_flag = immediate as u8;
                if immediate {
                    insert.splice_time = None;
                }
            }
            PatchPath::SpliceInsertSpliceEventId => {
                splice_insert_mut(&mut self.section)?.splice_event_id =
                    parse_u32_be("splice_insert.splice_event_id", value)?;
            }
            PatchPath::SpliceInsertCancel => {
                splice_insert_mut(&mut self.section)?.splice_event_cancel_indicator =
                    parse_bool("splice_insert.cancel", value)? as u8;
            }
            PatchPath::SpliceInsertProgramSplice => {
                let flag = parse_bool("splice_insert.program_splice", value)?;
                let insert = splice_insert_mut(&mut self.section)?;
                insert.program_splice_flag = flag as u8;
                if flag {
                    insert.component_count = 0;
                    insert.components.clear();
                }
            }
            PatchPath::SpliceInsertSpliceTimePtsTime => {
                let pts = parse_u64("splice_insert.splice_time.pts_time", value)? & 0x1_FFFF_FFFF;
                let insert = splice_insert_mut(&mut self.section)?;
                insert.splice_immediate_flag = 0;
                insert.splice_time = Some(scte35::SpliceTime {
                    time_specified_flag: 1,
                    pts_time: Some(pts),
                });
            }
            PatchPath::SpliceInsertSpliceTimeImmediate => {
                let immediate = parse_bool("splice_insert.splice_time.immediate", value)?;
                let insert = splice_insert_mut(&mut self.section)?;
                insert.splice_immediate_flag = immediate as u8;
                if immediate {
                    insert.splice_time = None;
                }
            }
            PatchPath::SpliceInsertDuration => {
                let ticks = parse_u64("splice_insert.duration", value)? & 0x1_FFFF_FFFF;
                let insert = splice_insert_mut(&mut self.section)?;
                insert.duration_flag = 1;
                insert.break_duration = Some(scte35::BreakDuration {
                    auto_return: 1,
                    reserved: 0x3F,
                    duration: ticks,
                });
            }
            PatchPath::SpliceInsertDurationClear => {
                let clear = parse_bool("splice_insert.duration.clear", value)?;
                if clear {
                    let insert = splice_insert_mut(&mut self.section)?;
                    insert.duration_flag = 0;
                    insert.break_duration = None;
                }
            }
            PatchPath::SpliceInsertAutoReturn => {
                let auto_return = parse_bool("splice_insert.auto_return", value)?;
                let insert = splice_insert_mut(&mut self.section)?;
                if let Some(duration) = insert.break_duration.as_mut() {
                    duration.auto_return = auto_return as u8;
                } else {
                    return Err("splice_insert.auto_return requires splice_insert.duration".into());
                }
            }
            PatchPath::SpliceInsertUniqueProgramId => {
                splice_insert_mut(&mut self.section)?.unique_program_id =
                    parse_u16("splice_insert.unique_program_id", value)?;
            }
            PatchPath::SpliceInsertAvailNum => {
                splice_insert_mut(&mut self.section)?.avail_num =
                    parse_u8("splice_insert.avail_num", value)?;
            }
            PatchPath::SpliceInsertAvailsExpected => {
                splice_insert_mut(&mut self.section)?.avails_expected =
                    parse_u8("splice_insert.avails_expected", value)?;
            }
            PatchPath::SpliceInsertComponentAdd => {
                let insert = splice_insert_mut(&mut self.section)?;
                let component = parse_splice_insert_component(value)?;
                insert.components.push(component);
                insert.component_count = insert.components.len() as u8;
            }
            PatchPath::SpliceInsertComponentClear => {
                let clear = parse_bool("splice_insert.component.clear", value)?;
                if clear {
                    let insert = splice_insert_mut(&mut self.section)?;
                    insert.components.clear();
                    insert.component_count = 0;
                }
            }
            PatchPath::SpliceInsertComponent { index, field } => {
                let insert = splice_insert_mut(&mut self.section)?;
                let component = insert
                    .components
                    .get_mut(*index)
                    .ok_or_else(|| format!("splice_insert.component[{index}] not found"))?;
                match field {
                    SpliceInsertComponentField::Tag => {
                        component.component_tag = parse_u8("splice_insert.component.tag", value)?;
                    }
                    SpliceInsertComponentField::PtsTime => {
                        let pts =
                            parse_u64("splice_insert.component.pts_time", value)? & 0x1_FFFF_FFFF;
                        component.splice_time = Some(scte35::SpliceTime {
                            time_specified_flag: 1,
                            pts_time: Some(pts),
                        });
                    }
                    SpliceInsertComponentField::Immediate => {
                        let immediate = parse_bool("splice_insert.component.immediate", value)?;
                        if immediate {
                            component.splice_time = None;
                        }
                    }
                }
            }
            PatchPath::SpliceScheduleComponent { index, field } => {
                let schedule = splice_schedule_mut(&mut self.section)?;
                if schedule.component_list.is_empty() {
                    return Err("splice_schedule has no components".into());
                }
                let component = schedule
                    .component_list
                    .get_mut(*index)
                    .ok_or_else(|| format!("splice_schedule.component[{index}] not found"))?;
                match field {
                    SpliceScheduleComponentField::Tag => {
                        component.component_tag = parse_u8("splice_schedule.component.tag", value)?;
                    }
                    SpliceScheduleComponentField::SpliceMode => {
                        component.splice_mode_indicator =
                            parse_u8("splice_schedule.component.splice_mode", value)?;
                    }
                    SpliceScheduleComponentField::Duration => {
                        component.splice_duration =
                            Some(parse_u32("splice_schedule.component.duration", value)?);
                        component.duration_flag = 1;
                        component.utc_splice_time = None;
                    }
                    SpliceScheduleComponentField::UtcSpliceTime => {
                        component.utc_splice_time = Some(parse_u32(
                            "splice_schedule.component.utc_splice_time",
                            value,
                        )?);
                        component.duration_flag = 0;
                        component.splice_duration = None;
                    }
                    SpliceScheduleComponentField::DurationFlag => {
                        component.duration_flag =
                            parse_bool("splice_schedule.component.duration_flag", value)? as u8;
                    }
                }
            }
            PatchPath::SegmentationEventId => {
                segmentation_mut(&mut self.section)?.segmentation_event_id =
                    parse_u32_be("segmentation.event_id", value)?;
            }
            PatchPath::SegmentationCancel => {
                segmentation_mut(&mut self.section)?.segmentation_event_cancel_indicator =
                    parse_bool("segmentation.cancel", value)?;
            }
            PatchPath::SegmentationProgram => {
                segmentation_mut(&mut self.section)?.program_segmentation_flag =
                    parse_bool("segmentation.program", value)?;
            }
            PatchPath::SegmentationDuration => {
                let duration = parse_u64("segmentation.duration", value)? & 0x1_FFFF_FFFF;
                let seg = segmentation_mut(&mut self.section)?;
                seg.segmentation_duration_flag = true;
                seg.segmentation_duration = Some(duration);
            }
            PatchPath::SegmentationDurationClear => {
                let clear = parse_bool("segmentation.duration.clear", value)?;
                if clear {
                    let seg = segmentation_mut(&mut self.section)?;
                    seg.segmentation_duration_flag = false;
                    seg.segmentation_duration = None;
                }
            }
            PatchPath::SegmentationDeliveryNotRestricted => {
                let flag = parse_bool("segmentation.delivery_not_restricted", value)?;
                let seg = segmentation_mut(&mut self.section)?;
                seg.delivery_not_restricted_flag = flag;
                if flag {
                    seg.web_delivery_allowed_flag = None;
                    seg.no_regional_blackout_flag = None;
                    seg.archive_allowed_flag = None;
                    seg.device_restrictions = None;
                }
            }
            PatchPath::SegmentationWebDeliveryAllowed => {
                segmentation_mut(&mut self.section)?.web_delivery_allowed_flag =
                    Some(parse_bool("segmentation.web_delivery_allowed", value)?);
            }
            PatchPath::SegmentationNoRegionalBlackout => {
                segmentation_mut(&mut self.section)?.no_regional_blackout_flag =
                    Some(parse_bool("segmentation.no_regional_blackout", value)?);
            }
            PatchPath::SegmentationArchiveAllowed => {
                segmentation_mut(&mut self.section)?.archive_allowed_flag =
                    Some(parse_bool("segmentation.archive_allowed", value)?);
            }
            PatchPath::SegmentationDeviceRestrictions => {
                segmentation_mut(&mut self.section)?.device_restrictions =
                    Some(parse_u8("segmentation.device_restrictions", value)?);
            }
            PatchPath::SegmentationUpidType => {
                let id = parse_u8("segmentation.upid_type", value)?;
                let seg = segmentation_mut(&mut self.section)?;
                seg.segmentation_upid_type = scte35::SegmentationUpidType::from(id);
            }
            PatchPath::SegmentationUpid => {
                let bytes = parse_bytes(value)?;
                let seg = segmentation_mut(&mut self.section)?;
                seg.segmentation_upid_length = bytes.len() as u8;
                seg.segmentation_upid = bytes;
            }
            PatchPath::SegmentationTypeId => {
                let id = parse_u8("segmentation.type_id", value)?;
                let seg = segmentation_mut(&mut self.section)?;
                seg.segmentation_type_id = id;
                seg.segmentation_type = scte35::SegmentationType::from_id(id);
            }
            PatchPath::SegmentationSegmentNum => {
                segmentation_mut(&mut self.section)?.segment_num =
                    parse_u8("segmentation.segment_num", value)?;
            }
            PatchPath::SegmentationSegmentsExpected => {
                segmentation_mut(&mut self.section)?.segments_expected =
                    parse_u8("segmentation.segments_expected", value)?;
            }
            PatchPath::SegmentationSubSegmentNum => {
                let id = parse_u8("segmentation.sub_segment_num", value)?;
                let seg = segmentation_mut(&mut self.section)?;
                seg.sub_segment_num = Some(id);
                if seg.sub_segments_expected.is_none() {
                    seg.sub_segments_expected = Some(0);
                }
            }
            PatchPath::SegmentationSubSegmentsExpected => {
                let id = parse_u8("segmentation.sub_segments_expected", value)?;
                let seg = segmentation_mut(&mut self.section)?;
                seg.sub_segments_expected = Some(id);
                if seg.sub_segment_num.is_none() {
                    seg.sub_segment_num = Some(0);
                }
            }
            PatchPath::SegmentationSubSegmentClear => {
                let clear = parse_bool("segmentation.sub_segment.clear", value)?;
                if clear {
                    let seg = segmentation_mut(&mut self.section)?;
                    seg.sub_segment_num = None;
                    seg.sub_segments_expected = None;
                }
            }
            PatchPath::AvailProviderId => {
                let bytes = parse_bytes(value)?;
                avail_mut(&mut self.section)?.provider_avail_id = bytes;
            }
            PatchPath::AvailIdentifier => {
                avail_mut(&mut self.section)?.identifier = parse_u32("avail.identifier", value)?;
            }
            PatchPath::DtmfPreroll => {
                dtmf_mut(&mut self.section)?.preroll = parse_u8("dtmf.preroll", value)?;
            }
            PatchPath::DtmfChars => {
                let bytes = parse_bytes(value)?;
                let dtmf = dtmf_mut(&mut self.section)?;
                dtmf.dtmf_count = bytes.len() as u8;
                dtmf.dtmf_chars = bytes;
            }
            PatchPath::DtmfIdentifier => {
                dtmf_mut(&mut self.section)?.identifier = parse_u32("dtmf.identifier", value)?;
            }
            PatchPath::TimeTaiSeconds => {
                time_desc_mut(&mut self.section)?.tai_seconds = parse_bytes(value)?;
            }
            PatchPath::TimeTaiNs => {
                time_desc_mut(&mut self.section)?.tai_ns = parse_bytes(value)?;
            }
            PatchPath::TimeUtcOffset => {
                time_desc_mut(&mut self.section)?.utc_offset = parse_bytes(value)?;
            }
            PatchPath::TimeIdentifier => {
                time_desc_mut(&mut self.section)?.identifier = parse_u32("time.identifier", value)?;
            }
            PatchPath::AudioComponents => {
                audio_mut(&mut self.section)?.audio_components = parse_bytes(value)?;
            }
            PatchPath::AudioIdentifier => {
                audio_mut(&mut self.section)?.identifier = parse_u32("audio.identifier", value)?;
            }
            PatchPath::UnknownTag => {
                let tag = parse_u8("unknown.tag", value)?;
                let (tag_ref, _len_ref, _data_ref) = unknown_mut(&mut self.section)?;
                *tag_ref = tag;
            }
            PatchPath::UnknownData => {
                let bytes = parse_bytes(value)?;
                let (_tag_ref, len_ref, data_ref) = unknown_mut(&mut self.section)?;
                *len_ref = bytes.len() as u8;
                *data_ref = bytes;
            }
            PatchPath::Descriptor { kind, index, field } => match kind {
                DescriptorKind::Segmentation => {
                    apply_segmentation_field(self, *index, *field, value)?
                }
                DescriptorKind::Avail => apply_avail_field(self, *index, *field, value)?,
                DescriptorKind::Dtmf => apply_dtmf_field(self, *index, *field, value)?,
                DescriptorKind::Time => apply_time_field(self, *index, *field, value)?,
                DescriptorKind::Audio => apply_audio_field(self, *index, *field, value)?,
                DescriptorKind::Unknown => apply_unknown_field(self, *index, *field, value)?,
            },
        }
        Ok(())
    }
}

fn time_signal_mut(section: &mut SpliceInfoSection) -> Result<&mut scte35::SpliceTime, String> {
    match &mut section.splice_command {
        scte35::SpliceCommand::TimeSignal(signal) => Ok(&mut signal.splice_time),
        _ => Err("splice_time is only supported for TimeSignal commands".into()),
    }
}

fn splice_insert_mut(section: &mut SpliceInfoSection) -> Result<&mut scte35::SpliceInsert, String> {
    match &mut section.splice_command {
        scte35::SpliceCommand::SpliceInsert(insert) => Ok(insert),
        _ => Err("splice_insert paths are only supported for SpliceInsert commands".into()),
    }
}

fn splice_schedule_mut(
    section: &mut SpliceInfoSection,
) -> Result<&mut scte35::SpliceSchedule, String> {
    match &mut section.splice_command {
        scte35::SpliceCommand::SpliceSchedule(schedule) => Ok(schedule),
        _ => Err("splice_schedule paths are only supported for SpliceSchedule commands".into()),
    }
}

fn segmentation_mut(
    section: &mut SpliceInfoSection,
) -> Result<&mut scte35::SegmentationDescriptor, String> {
    let index = section
        .splice_descriptors
        .iter()
        .position(|descriptor| matches!(descriptor, scte35::SpliceDescriptor::Segmentation(_)));

    let idx = match index {
        Some(idx) => idx,
        None => {
            let seg = scte35::SegmentationDescriptor {
                segmentation_event_id: 1,
                segmentation_event_cancel_indicator: false,
                program_segmentation_flag: true,
                segmentation_duration_flag: false,
                delivery_not_restricted_flag: true,
                web_delivery_allowed_flag: None,
                no_regional_blackout_flag: None,
                archive_allowed_flag: None,
                device_restrictions: None,
                segmentation_duration: None,
                segmentation_upid_type: scte35::SegmentationUpidType::NotUsed,
                segmentation_upid_length: 0,
                segmentation_upid: Vec::new(),
                segmentation_type_id: 0,
                segmentation_type: scte35::SegmentationType::NotIndicated,
                segment_num: 0,
                segments_expected: 0,
                sub_segment_num: None,
                sub_segments_expected: None,
            };
            section
                .splice_descriptors
                .push(scte35::SpliceDescriptor::Segmentation(seg));
            section.splice_descriptors.len() - 1
        }
    };

    match section.splice_descriptors.get_mut(idx) {
        Some(scte35::SpliceDescriptor::Segmentation(seg)) => Ok(seg),
        _ => Err("failed to access segmentation descriptor".into()),
    }
}

fn avail_mut(section: &mut SpliceInfoSection) -> Result<&mut AvailDescriptor, String> {
    descriptor_mut(section, DescriptorKind::Avail).and_then(|desc| match desc {
        scte35::SpliceDescriptor::Avail(avail) => Ok(avail),
        _ => Err("failed to access avail descriptor".into()),
    })
}

fn dtmf_mut(section: &mut SpliceInfoSection) -> Result<&mut DtmfDescriptor, String> {
    descriptor_mut(section, DescriptorKind::Dtmf).and_then(|desc| match desc {
        scte35::SpliceDescriptor::Dtmf(dtmf) => Ok(dtmf),
        _ => Err("failed to access dtmf descriptor".into()),
    })
}

fn time_desc_mut(section: &mut SpliceInfoSection) -> Result<&mut TimeDescriptor, String> {
    descriptor_mut(section, DescriptorKind::Time).and_then(|desc| match desc {
        scte35::SpliceDescriptor::Time(time_desc) => Ok(time_desc),
        _ => Err("failed to access time descriptor".into()),
    })
}

fn audio_mut(section: &mut SpliceInfoSection) -> Result<&mut AudioDescriptor, String> {
    descriptor_mut(section, DescriptorKind::Audio).and_then(|desc| match desc {
        scte35::SpliceDescriptor::Audio(audio) => Ok(audio),
        _ => Err("failed to access audio descriptor".into()),
    })
}

fn unknown_mut(
    section: &mut SpliceInfoSection,
) -> Result<(&mut u8, &mut u8, &mut Vec<u8>), String> {
    descriptor_mut(section, DescriptorKind::Unknown).and_then(|desc| match desc {
        scte35::SpliceDescriptor::Unknown { tag, length, data } => Ok((tag, length, data)),
        _ => Err("failed to access unknown descriptor".into()),
    })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DescriptorKind {
    Avail,
    Dtmf,
    Time,
    Audio,
    Unknown,
    Segmentation,
}

fn descriptor_mut(
    section: &mut SpliceInfoSection,
    kind: DescriptorKind,
) -> Result<&mut scte35::SpliceDescriptor, String> {
    let index =
        section
            .splice_descriptors
            .iter()
            .position(|descriptor| match (descriptor, &kind) {
                (scte35::SpliceDescriptor::Segmentation(_), DescriptorKind::Segmentation) => true,
                (scte35::SpliceDescriptor::Avail(_), DescriptorKind::Avail) => true,
                (scte35::SpliceDescriptor::Dtmf(_), DescriptorKind::Dtmf) => true,
                (scte35::SpliceDescriptor::Time(_), DescriptorKind::Time) => true,
                (scte35::SpliceDescriptor::Audio(_), DescriptorKind::Audio) => true,
                (scte35::SpliceDescriptor::Unknown { .. }, DescriptorKind::Unknown) => true,
                _ => false,
            });

    let idx = match index {
        Some(idx) => idx,
        None => {
            let descriptor = match kind {
                DescriptorKind::Segmentation => {
                    scte35::SpliceDescriptor::Segmentation(scte35::SegmentationDescriptor {
                        segmentation_event_id: 1,
                        segmentation_event_cancel_indicator: false,
                        program_segmentation_flag: true,
                        segmentation_duration_flag: false,
                        delivery_not_restricted_flag: true,
                        web_delivery_allowed_flag: None,
                        no_regional_blackout_flag: None,
                        archive_allowed_flag: None,
                        device_restrictions: None,
                        segmentation_duration: None,
                        segmentation_upid_type: scte35::SegmentationUpidType::NotUsed,
                        segmentation_upid_length: 0,
                        segmentation_upid: Vec::new(),
                        segmentation_type_id: 0,
                        segmentation_type: scte35::SegmentationType::NotIndicated,
                        segment_num: 0,
                        segments_expected: 0,
                        sub_segment_num: None,
                        sub_segments_expected: None,
                    })
                }
                DescriptorKind::Avail => scte35::SpliceDescriptor::Avail(AvailDescriptor {
                    identifier: 0x4355_4549,
                    provider_avail_id: Vec::new(),
                }),
                DescriptorKind::Dtmf => scte35::SpliceDescriptor::Dtmf(DtmfDescriptor {
                    identifier: 0x4355_4549,
                    preroll: 0,
                    dtmf_count: 0,
                    dtmf_chars: Vec::new(),
                }),
                DescriptorKind::Time => scte35::SpliceDescriptor::Time(TimeDescriptor {
                    identifier: 0x4355_4549,
                    tai_seconds: Vec::new(),
                    tai_ns: Vec::new(),
                    utc_offset: Vec::new(),
                }),
                DescriptorKind::Audio => scte35::SpliceDescriptor::Audio(AudioDescriptor {
                    identifier: 0x4355_4549,
                    audio_components: Vec::new(),
                }),
                DescriptorKind::Unknown => scte35::SpliceDescriptor::Unknown {
                    tag: 0,
                    length: 0,
                    data: Vec::new(),
                },
            };
            section.splice_descriptors.push(descriptor);
            section.splice_descriptors.len() - 1
        }
    };

    section
        .splice_descriptors
        .get_mut(idx)
        .ok_or_else(|| "failed to access descriptor".into())
}

fn descriptor_at(
    section: &mut SpliceInfoSection,
    kind: DescriptorKind,
    index: usize,
) -> Result<&mut scte35::SpliceDescriptor, String> {
    let matches_kind = |descriptor: &scte35::SpliceDescriptor| match (descriptor, &kind) {
        (scte35::SpliceDescriptor::Segmentation(_), DescriptorKind::Segmentation) => true,
        (scte35::SpliceDescriptor::Avail(_), DescriptorKind::Avail) => true,
        (scte35::SpliceDescriptor::Dtmf(_), DescriptorKind::Dtmf) => true,
        (scte35::SpliceDescriptor::Time(_), DescriptorKind::Time) => true,
        (scte35::SpliceDescriptor::Audio(_), DescriptorKind::Audio) => true,
        (scte35::SpliceDescriptor::Unknown { .. }, DescriptorKind::Unknown) => true,
        _ => false,
    };

    let mut indices = section
        .splice_descriptors
        .iter()
        .enumerate()
        .filter(|(_, d)| matches_kind(d))
        .map(|(i, _)| i);

    let idx = indices
        .nth(index)
        .ok_or_else(|| format!("descriptor[{index}] not found for {kind:?}"))?;

    section
        .splice_descriptors
        .get_mut(idx)
        .ok_or_else(|| "failed to access descriptor".into())
}

fn apply_segmentation_field(
    doc: &mut Scte35Document,
    index: usize,
    field: DescriptorField,
    value: &str,
) -> Result<(), String> {
    ensure_descriptor_index(&mut doc.section, DescriptorKind::Segmentation, index)?;
    let desc = descriptor_at(&mut doc.section, DescriptorKind::Segmentation, index)?;
    let seg = match desc {
        scte35::SpliceDescriptor::Segmentation(seg) => seg,
        _ => return Err("descriptor type mismatch".into()),
    };
    match field {
        DescriptorField::SegmentationEventId => {
            seg.segmentation_event_id = parse_u32_be("segmentation.event_id", value)?;
        }
        DescriptorField::SegmentationCancel => {
            seg.segmentation_event_cancel_indicator = parse_bool("segmentation.cancel", value)?;
        }
        DescriptorField::SegmentationProgram => {
            seg.program_segmentation_flag = parse_bool("segmentation.program", value)?;
        }
        DescriptorField::SegmentationDuration => {
            let duration = parse_u64("segmentation.duration", value)? & 0x1_FFFF_FFFF;
            seg.segmentation_duration_flag = true;
            seg.segmentation_duration = Some(duration);
        }
        DescriptorField::SegmentationDurationClear => {
            let clear = parse_bool("segmentation.duration.clear", value)?;
            if clear {
                seg.segmentation_duration_flag = false;
                seg.segmentation_duration = None;
            }
        }
        DescriptorField::SegmentationDeliveryNotRestricted => {
            let flag = parse_bool("segmentation.delivery_not_restricted", value)?;
            seg.delivery_not_restricted_flag = flag;
            if flag {
                seg.web_delivery_allowed_flag = None;
                seg.no_regional_blackout_flag = None;
                seg.archive_allowed_flag = None;
                seg.device_restrictions = None;
            }
        }
        DescriptorField::SegmentationWebDeliveryAllowed => {
            seg.web_delivery_allowed_flag =
                Some(parse_bool("segmentation.web_delivery_allowed", value)?);
        }
        DescriptorField::SegmentationNoRegionalBlackout => {
            seg.no_regional_blackout_flag =
                Some(parse_bool("segmentation.no_regional_blackout", value)?);
        }
        DescriptorField::SegmentationArchiveAllowed => {
            seg.archive_allowed_flag = Some(parse_bool("segmentation.archive_allowed", value)?);
        }
        DescriptorField::SegmentationDeviceRestrictions => {
            seg.device_restrictions = Some(parse_u8("segmentation.device_restrictions", value)?);
        }
        DescriptorField::SegmentationUpidType => {
            let id = parse_u8("segmentation.upid_type", value)?;
            seg.segmentation_upid_type = scte35::SegmentationUpidType::from(id);
        }
        DescriptorField::SegmentationUpid => {
            let bytes = parse_bytes(value)?;
            seg.segmentation_upid_length = bytes.len() as u8;
            seg.segmentation_upid = bytes;
        }
        DescriptorField::SegmentationTypeId => {
            let id = parse_u8("segmentation.type_id", value)?;
            seg.segmentation_type_id = id;
            seg.segmentation_type = scte35::SegmentationType::from_id(id);
        }
        DescriptorField::SegmentationSegmentNum => {
            seg.segment_num = parse_u8("segmentation.segment_num", value)?;
        }
        DescriptorField::SegmentationSegmentsExpected => {
            seg.segments_expected = parse_u8("segmentation.segments_expected", value)?;
        }
        DescriptorField::SegmentationSubSegmentNum => {
            let id = parse_u8("segmentation.sub_segment_num", value)?;
            seg.sub_segment_num = Some(id);
            if seg.sub_segments_expected.is_none() {
                seg.sub_segments_expected = Some(0);
            }
        }
        DescriptorField::SegmentationSubSegmentsExpected => {
            let id = parse_u8("segmentation.sub_segments_expected", value)?;
            seg.sub_segments_expected = Some(id);
            if seg.sub_segment_num.is_none() {
                seg.sub_segment_num = Some(0);
            }
        }
        DescriptorField::SegmentationSubSegmentClear => {
            let clear = parse_bool("segmentation.sub_segment.clear", value)?;
            if clear {
                seg.sub_segment_num = None;
                seg.sub_segments_expected = None;
            }
        }
        _ => return Err("unsupported segmentation field".into()),
    }
    Ok(())
}

fn apply_avail_field(
    doc: &mut Scte35Document,
    index: usize,
    field: DescriptorField,
    value: &str,
) -> Result<(), String> {
    ensure_descriptor_index(&mut doc.section, DescriptorKind::Avail, index)?;
    let desc = descriptor_at(&mut doc.section, DescriptorKind::Avail, index)?;
    let avail = match desc {
        scte35::SpliceDescriptor::Avail(avail) => avail,
        _ => return Err("descriptor type mismatch".into()),
    };
    match field {
        DescriptorField::AvailProviderId => {
            avail.provider_avail_id = parse_bytes(value)?;
        }
        DescriptorField::AvailIdentifier => {
            avail.identifier = parse_u32("avail.identifier", value)?;
        }
        _ => return Err("unsupported avail field".into()),
    }
    Ok(())
}

fn apply_dtmf_field(
    doc: &mut Scte35Document,
    index: usize,
    field: DescriptorField,
    value: &str,
) -> Result<(), String> {
    ensure_descriptor_index(&mut doc.section, DescriptorKind::Dtmf, index)?;
    let desc = descriptor_at(&mut doc.section, DescriptorKind::Dtmf, index)?;
    let dtmf = match desc {
        scte35::SpliceDescriptor::Dtmf(dtmf) => dtmf,
        _ => return Err("descriptor type mismatch".into()),
    };
    match field {
        DescriptorField::DtmfPreroll => {
            dtmf.preroll = parse_u8("dtmf.preroll", value)?;
        }
        DescriptorField::DtmfChars => {
            let bytes = parse_bytes(value)?;
            dtmf.dtmf_count = bytes.len() as u8;
            dtmf.dtmf_chars = bytes;
        }
        DescriptorField::DtmfIdentifier => {
            dtmf.identifier = parse_u32("dtmf.identifier", value)?;
        }
        _ => return Err("unsupported dtmf field".into()),
    }
    Ok(())
}

fn apply_time_field(
    doc: &mut Scte35Document,
    index: usize,
    field: DescriptorField,
    value: &str,
) -> Result<(), String> {
    ensure_descriptor_index(&mut doc.section, DescriptorKind::Time, index)?;
    let desc = descriptor_at(&mut doc.section, DescriptorKind::Time, index)?;
    let time_desc = match desc {
        scte35::SpliceDescriptor::Time(time_desc) => time_desc,
        _ => return Err("descriptor type mismatch".into()),
    };
    match field {
        DescriptorField::TimeTaiSeconds => {
            time_desc.tai_seconds = parse_bytes(value)?;
        }
        DescriptorField::TimeTaiNs => {
            time_desc.tai_ns = parse_bytes(value)?;
        }
        DescriptorField::TimeUtcOffset => {
            time_desc.utc_offset = parse_bytes(value)?;
        }
        DescriptorField::TimeIdentifier => {
            time_desc.identifier = parse_u32("time.identifier", value)?;
        }
        _ => return Err("unsupported time field".into()),
    }
    Ok(())
}

fn apply_audio_field(
    doc: &mut Scte35Document,
    index: usize,
    field: DescriptorField,
    value: &str,
) -> Result<(), String> {
    ensure_descriptor_index(&mut doc.section, DescriptorKind::Audio, index)?;
    let desc = descriptor_at(&mut doc.section, DescriptorKind::Audio, index)?;
    let audio = match desc {
        scte35::SpliceDescriptor::Audio(audio) => audio,
        _ => return Err("descriptor type mismatch".into()),
    };
    match field {
        DescriptorField::AudioComponents => {
            audio.audio_components = parse_bytes(value)?;
        }
        DescriptorField::AudioIdentifier => {
            audio.identifier = parse_u32("audio.identifier", value)?;
        }
        _ => return Err("unsupported audio field".into()),
    }
    Ok(())
}

fn apply_unknown_field(
    doc: &mut Scte35Document,
    index: usize,
    field: DescriptorField,
    value: &str,
) -> Result<(), String> {
    ensure_descriptor_index(&mut doc.section, DescriptorKind::Unknown, index)?;
    let desc = descriptor_at(&mut doc.section, DescriptorKind::Unknown, index)?;
    let (tag, length, data) = match desc {
        scte35::SpliceDescriptor::Unknown { tag, length, data } => (tag, length, data),
        _ => return Err("descriptor type mismatch".into()),
    };
    match field {
        DescriptorField::UnknownTag => {
            *tag = parse_u8("unknown.tag", value)?;
        }
        DescriptorField::UnknownData => {
            let bytes = parse_bytes(value)?;
            *length = bytes.len() as u8;
            *data = bytes;
        }
        _ => return Err("unsupported unknown field".into()),
    }
    Ok(())
}

fn remove_descriptor_at(
    section: &mut SpliceInfoSection,
    kind: DescriptorKind,
    index: usize,
) -> Result<(), String> {
    let matches_kind = |descriptor: &scte35::SpliceDescriptor| match (descriptor, &kind) {
        (scte35::SpliceDescriptor::Segmentation(_), DescriptorKind::Segmentation) => true,
        (scte35::SpliceDescriptor::Avail(_), DescriptorKind::Avail) => true,
        (scte35::SpliceDescriptor::Dtmf(_), DescriptorKind::Dtmf) => true,
        (scte35::SpliceDescriptor::Time(_), DescriptorKind::Time) => true,
        (scte35::SpliceDescriptor::Audio(_), DescriptorKind::Audio) => true,
        (scte35::SpliceDescriptor::Unknown { .. }, DescriptorKind::Unknown) => true,
        _ => false,
    };

    let mut indices = section
        .splice_descriptors
        .iter()
        .enumerate()
        .filter(|(_, d)| matches_kind(d))
        .map(|(i, _)| i);

    let idx = indices
        .nth(index)
        .ok_or_else(|| format!("descriptor[{index}] not found for {kind:?}"))?;

    section.splice_descriptors.remove(idx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::time::Duration;

    #[test]
    fn summary_reports_fields() {
        let doc = Scte35Document::new_default_time_signal().expect("doc");
        let summary = doc.summary();
        assert_eq!(summary.table_id, doc.section.table_id);
        assert_eq!(summary.pts_adjustment, doc.section.pts_adjustment);
        assert_eq!(summary.tier, doc.section.tier);
        assert_eq!(summary.cw_index, doc.section.cw_index);
        assert_eq!(summary.splice_command, "time_signal");
    }

    #[test]
    fn parse_auto_detect_error_on_unknown_format() {
        let err = Scte35Document::parse(
            "not-json-and-not-hex-or-base64",
            InputFormat::Auto,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect_err("expected error");
        assert!(err.contains("unable to detect input format"));
    }

    #[test]
    fn new_default_variants_build() {
        Scte35Document::new_default_splice_null().expect("splice_null");
        Scte35Document::new_default_splice_insert().expect("splice_insert");
        Scte35Document::new_default_splice_schedule().expect("splice_schedule");
    }

    #[test]
    fn validate_patch_value_component_spec_errors() {
        let err = validate_patch_value("splice_schedule.component.add", "tag=1")
            .expect_err("expected error");
        assert!(err.contains("splice_schedule components are not supported"));

        let err = validate_patch_value("splice_insert.component.add", "tag=1,pts=90000,foo=1")
            .expect_err("expected error");
        assert!(err.contains("unknown component field"));

        let err = validate_patch_value("splice_insert.component[0].bogus", "1")
            .expect_err("expected error");
        assert!(err.contains("unsupported path"));
    }

    #[test]
    fn delete_target_descriptor_not_found_errors() {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        let err = doc.delete_target("audio[0]").expect_err("expected error");
        assert!(err.contains("descriptor"));
    }

    #[test]
    fn apply_splice_command_variants() {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        doc.apply_sets(&[(
            "splice_command".to_string(),
            "bandwidth_reservation".to_string(),
        )])
        .expect("bandwidth");
        doc.apply_sets(&[("splice_command".to_string(), "private_command".to_string())])
            .expect("private");

        let err = doc
            .apply_sets(&[("splice_command".to_string(), "unknown".to_string())])
            .expect_err("expected error");
        assert!(err.contains("splice_command must be one of"));
    }

    #[test]
    fn splice_insert_auto_return_requires_duration() {
        let mut doc = Scte35Document::new_default_splice_insert().expect("doc");
        doc.apply_sets(&[(
            "splice_insert.duration.clear".to_string(),
            "true".to_string(),
        )])
        .expect("clear");
        let err = doc
            .apply_sets(&[("splice_insert.auto_return".to_string(), "true".to_string())])
            .expect_err("expected error");
        assert!(err.contains("requires splice_insert.duration"));
    }

    #[test]
    fn splice_schedule_component_add_not_supported() {
        let mut doc = Scte35Document::new_default_splice_schedule().expect("doc");
        let err = doc
            .apply_sets(&[(
                "splice_schedule.component.add".to_string(),
                "tag=1".to_string(),
            )])
            .expect_err("expected error");
        assert!(err.contains("not supported"));
    }

    #[test]
    fn splice_schedule_component_empty_list_error() {
        let mut doc = Scte35Document::new_default_splice_schedule().expect("doc");
        let err = doc
            .apply_sets(&[(
                "splice_schedule.component[0].tag".to_string(),
                "1".to_string(),
            )])
            .expect_err("expected error");
        assert!(err.contains("splice_schedule has no components"));
    }

    #[test]
    fn segmentation_sub_segment_auto_fill() {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        doc.apply_sets(&[("segmentation.sub_segment_num".to_string(), "1".to_string())])
            .expect("sub_segment_num");
        doc.apply_sets(&[(
            "segmentation.sub_segments_expected".to_string(),
            "2".to_string(),
        )])
        .expect("sub_segments_expected");
    }

    #[test]
    fn apply_descriptor_indexed_unknown_fields_errors() {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        doc.apply_sets(&[("unknown[0].tag".to_string(), "1".to_string())])
            .expect("set unknown tag");
        doc.apply_sets(&[("unknown[0].data".to_string(), "0x01".to_string())])
            .expect("set unknown data");
        doc.apply_sets(&[("audio[0].identifier".to_string(), "1".to_string())])
            .expect("set audio identifier");
        doc.apply_sets(&[("time[0].identifier".to_string(), "1".to_string())])
            .expect("set time identifier");
        doc.apply_sets(&[("dtmf[0].identifier".to_string(), "1".to_string())])
            .expect("set dtmf identifier");
        doc.apply_sets(&[("avail[0].identifier".to_string(), "1".to_string())])
            .expect("set avail identifier");
    }

    #[test]
    fn descriptor_access_error_branches() {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        doc.apply_sets(&[("avail[0].identifier".to_string(), "1".to_string())])
            .expect("set avail identifier");
    }

    #[test]
    fn validate_patch_value_type_bounds() {
        let err = validate_patch_value("table_id", "999").expect_err("u8");
        assert!(err.contains("invalid value"));
        let err = validate_patch_value("tier", "999999").expect_err("u16");
        assert!(err.contains("invalid value"));
    }

    #[test]
    fn parse_bool_error() {
        let err = validate_patch_value("splice_insert.cancel", "not-bool").expect_err("bool");
        assert!(err.contains("expected true/false"));
    }

    #[test]
    fn parse_bytes_invalid() {
        let err = parse_bytes("###").expect_err("bytes");
        assert!(err.contains("invalid"));
    }

    #[test]
    fn parse_indexed_and_only_errors() {
        assert!(parse_indexed("splice_insert.component", "splice_insert.component").is_none());
        assert!(parse_index_only("splice_insert.component[]", "splice_insert.component").is_none());
    }

    #[test]
    fn descriptor_mut_creates_each_kind() {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        let _ = avail_mut(&mut doc.section).expect("avail");
        let _ = dtmf_mut(&mut doc.section).expect("dtmf");
        let _ = time_desc_mut(&mut doc.section).expect("time");
        let _ = audio_mut(&mut doc.section).expect("audio");
        let _ = unknown_mut(&mut doc.section).expect("unknown");
    }

    #[test]
    fn splice_time_immediate_requires_pts_time() {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        doc.apply_sets(&[("splice_time.immediate".to_string(), "true".to_string())])
            .expect("clear");
        let err = doc
            .apply_sets(&[("splice_time.immediate".to_string(), "false".to_string())])
            .expect_err("expected error");
        assert!(err.contains("requires splice_time.pts_time"));
    }

    fn time_signal_doc() -> Scte35Document {
        Scte35Document::new_default_time_signal().expect("time signal doc")
    }

    #[test]
    fn patch_meta_lookup() {
        let meta = patch_meta("table_id").expect("meta");
        assert_eq!(meta.path, "table_id");
        let meta_indexed = patch_meta("avail[0].provider_id").expect("meta");
        assert_eq!(meta_indexed.value_type, PatchValueType::Bytes);
    }

    #[test]
    fn validate_patch_value_errors_extra() {
        assert!(validate_patch_value("table_id", "abc").is_err());
        assert!(validate_patch_value("splice_command", "bogus").is_err());
        assert!(validate_patch_value("nope", "1").is_err());
    }

    #[test]
    fn apply_time_signal_paths() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            ("table_id".to_string(), "252".to_string()),
            ("pts_adjustment".to_string(), "90000".to_string()),
            ("tier".to_string(), "4095".to_string()),
            ("cw_index".to_string(), "1".to_string()),
            ("splice_time.pts_time".to_string(), "180000".to_string()),
            ("splice_time.immediate".to_string(), "true".to_string()),
        ])
        .expect("apply");
    }

    #[test]
    fn apply_splice_insert_paths() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        doc.apply_sets(&[
            (
                "splice_insert.splice_event_id".to_string(),
                "10".to_string(),
            ),
            ("splice_insert.cancel".to_string(), "false".to_string()),
            (
                "splice_insert.out_of_network".to_string(),
                "true".to_string(),
            ),
            (
                "splice_insert.program_splice".to_string(),
                "false".to_string(),
            ),
            (
                "splice_insert.splice_immediate".to_string(),
                "false".to_string(),
            ),
            (
                "splice_insert.splice_time.pts_time".to_string(),
                "90000".to_string(),
            ),
            ("splice_insert.duration".to_string(), "90000".to_string()),
            ("splice_insert.auto_return".to_string(), "true".to_string()),
            (
                "splice_insert.unique_program_id".to_string(),
                "7".to_string(),
            ),
            ("splice_insert.avail_num".to_string(), "1".to_string()),
            ("splice_insert.avails_expected".to_string(), "2".to_string()),
            (
                "splice_insert.component.add".to_string(),
                "tag=1,pts=90000".to_string(),
            ),
            (
                "splice_insert.component[0].tag".to_string(),
                "2".to_string(),
            ),
            (
                "splice_insert.component[0].pts_time".to_string(),
                "180000".to_string(),
            ),
            (
                "splice_insert.component[0].immediate".to_string(),
                "true".to_string(),
            ),
            (
                "splice_insert.component.clear".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn apply_splice_schedule_paths() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_schedule".to_string())])
            .expect("set command");
        {
            let schedule = splice_schedule_mut(&mut doc.section).expect("schedule");
            schedule.component_list.push(scte35::ComponentSplice {
                component_tag: 1,
                reserved: 0,
                splice_mode_indicator: 0,
                duration_flag: 0,
                splice_duration: None,
                utc_splice_time: Some(10),
            });
            schedule.num_splice = 1;
        }
        doc.apply_sets(&[
            (
                "splice_schedule.splice_event_id".to_string(),
                "1".to_string(),
            ),
            ("splice_schedule.cancel".to_string(), "false".to_string()),
            (
                "splice_schedule.out_of_network".to_string(),
                "false".to_string(),
            ),
            (
                "splice_schedule.utc_splice_time".to_string(),
                "100".to_string(),
            ),
            (
                "splice_schedule.duration.clear".to_string(),
                "true".to_string(),
            ),
            (
                "splice_schedule.unique_program_id".to_string(),
                "2".to_string(),
            ),
            (
                "splice_schedule.component[0].tag".to_string(),
                "2".to_string(),
            ),
            (
                "splice_schedule.component[0].splice_mode".to_string(),
                "1".to_string(),
            ),
            (
                "splice_schedule.component[0].duration".to_string(),
                "120".to_string(),
            ),
            (
                "splice_schedule.component[0].utc_splice_time".to_string(),
                "10".to_string(),
            ),
            (
                "splice_schedule.component[0].duration_flag".to_string(),
                "true".to_string(),
            ),
            (
                "splice_schedule.component.clear".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn apply_descriptor_paths_indexed() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            ("segmentation[0].event_id".to_string(), "5".to_string()),
            ("segmentation[0].type_id".to_string(), "48".to_string()),
            ("segmentation[0].upid".to_string(), "0x".to_string()),
            ("avail[0].provider_id".to_string(), "0x41424344".to_string()),
            ("dtmf[0].chars".to_string(), "0x313233".to_string()),
            (
                "time[0].tai_seconds".to_string(),
                "0x000000000001".to_string(),
            ),
            ("audio[0].components".to_string(), "0x1122".to_string()),
            ("unknown[0].data".to_string(), "0x010203".to_string()),
        ])
        .expect("apply");
    }

    #[test]
    fn delete_targets() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            ("avail.provider_id".to_string(), "0x41424344".to_string()),
            ("dtmf.chars".to_string(), "0x313233".to_string()),
        ])
        .expect("apply");
        doc.delete_target("avail[0]").expect("delete avail");
        doc.delete_target("dtmf[0]").expect("delete dtmf");
    }

    #[test]
    fn error_branches() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_time.immediate".to_string(), "true".to_string())])
            .expect("set immediate true");
        let err = doc
            .apply_sets(&[("splice_time.immediate".to_string(), "false".to_string())])
            .expect_err("expected error");
        assert!(err.contains("requires splice_time.pts_time"));

        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        let err = doc
            .apply_sets(&[("splice_insert.auto_return".to_string(), "true".to_string())])
            .expect_err("expected error");
        assert!(err.contains("requires splice_insert.duration"));
    }

    #[test]
    fn parse_helpers_cover_errors() {
        assert!(parse_u8("table_id", "nope").is_err());
        assert!(parse_u16("tier", "bad").is_err());
        assert!(parse_u32("id", "bad").is_err());
        assert!(parse_u64("pts", "bad").is_err());
        assert!(parse_bool("flag", "maybe").is_err());
        assert_eq!(parse_u32_be("id", "2").unwrap(), 2u32 << 24);
    }

    #[test]
    fn validate_patch_value_component_indexed_fields() {
        validate_patch_value("splice_insert.component[0].tag", "1").expect("ok");
        validate_patch_value("splice_insert.component[0].pts_time", "90000").expect("ok");
        validate_patch_value("splice_insert.component[0].immediate", "true").expect("ok");
        validate_patch_value("splice_schedule.component[0].tag", "1").expect("ok");
        validate_patch_value("splice_schedule.component[0].splice_mode", "1").expect("ok");
        validate_patch_value("splice_schedule.component[0].duration", "10").expect("ok");
        validate_patch_value("splice_schedule.component[0].utc_splice_time", "10").expect("ok");
        validate_patch_value("splice_schedule.component[0].duration_flag", "true").expect("ok");
    }

    #[test]
    fn parse_bytes_variants() {
        assert_eq!(parse_bytes("").unwrap(), Vec::<u8>::new());
        assert_eq!(parse_bytes("0x").unwrap(), Vec::<u8>::new());
        assert_eq!(parse_bytes("0x0102").unwrap(), vec![1, 2]);
        assert_eq!(parse_bytes("0102").unwrap(), vec![1, 2]);
        assert_eq!(parse_bytes("AQI=").unwrap(), vec![1, 2]);
        assert!(parse_bytes("not_base64").is_err());
    }

    #[test]
    fn parse_splice_insert_component_errors() {
        let err = parse_splice_insert_component("tag").expect_err("bad spec");
        assert!(err.contains("invalid component spec"));
        let err = parse_splice_insert_component("pts=1").expect_err("missing tag");
        assert!(err.contains("component requires tag"));
        let err = parse_splice_insert_component("tag=1,foo=2").expect_err("unknown field");
        assert!(err.contains("unknown component field"));
    }

    #[test]
    fn parse_splice_insert_component_immediate_clears_pts() {
        let component =
            parse_splice_insert_component("tag=1,pts=90000,immediate=true").expect("component");
        assert!(component.splice_time.is_none());
    }

    #[test]
    fn parse_patch_path_errors() {
        let err = parse_patch_path("segmentation[0]").expect_err("missing field");
        assert!(err.contains("unsupported path"));
        let err = parse_patch_path("segmentation[0].nope").expect_err("bad field");
        assert!(err.contains("unsupported path"));
    }

    #[test]
    fn normalize_indexed_path_matches() {
        assert_eq!(
            normalize_indexed_path("segmentation[12].event_id"),
            Some("segmentation[i].event_id")
        );
        assert_eq!(normalize_indexed_path("nope[1].value"), None);
    }

    #[test]
    fn validate_patch_value_errors() {
        let err = validate_patch_value("splice_command", "nope").expect_err("bad command");
        assert!(err.contains("splice_command must be one of"));
        let err = validate_patch_value("splice_schedule.component.add", "tag=1").expect_err("bad");
        assert!(err.contains("not supported by encoder"));
        let err =
            validate_patch_value("splice_insert.component[0].bogus", "1").expect_err("bad path");
        assert!(err.contains("unsupported path"));
    }

    #[test]
    fn splice_schedule_component_errors() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_schedule".to_string())])
            .expect("set command");
        let err = doc
            .apply_sets(&[(
                "splice_schedule.component[0].tag".to_string(),
                "1".to_string(),
            )])
            .expect_err("no components");
        assert!(err.contains("splice_schedule has no components"));
    }

    #[test]
    fn splice_insert_paths_errors() {
        let mut doc = time_signal_doc();
        let err = doc
            .apply_sets(&[("splice_insert.avail_num".to_string(), "1".to_string())])
            .expect_err("wrong command");
        assert!(err.contains("splice_insert paths are only supported"));
        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        let err = doc
            .apply_sets(&[(
                "splice_insert.component[0].tag".to_string(),
                "1".to_string(),
            )])
            .expect_err("missing component");
        assert!(err.contains("splice_insert.component[0] not found"));
    }

    #[test]
    fn splice_insert_splice_time_not_time_signal() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        let err = doc
            .apply_sets(&[("splice_time.pts_time".to_string(), "1".to_string())])
            .expect_err("wrong command");
        assert!(err.contains("splice_time is only supported"));
    }

    #[test]
    fn segmentation_flag_paths() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            (
                "segmentation[0].delivery_not_restricted".to_string(),
                "false".to_string(),
            ),
            (
                "segmentation[0].web_delivery_allowed".to_string(),
                "true".to_string(),
            ),
            (
                "segmentation[0].no_regional_blackout".to_string(),
                "true".to_string(),
            ),
            (
                "segmentation[0].archive_allowed".to_string(),
                "false".to_string(),
            ),
            (
                "segmentation[0].device_restrictions".to_string(),
                "3".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn segmentation_sub_segment_clear() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            (
                "segmentation[0].sub_segment_num".to_string(),
                "1".to_string(),
            ),
            (
                "segmentation[0].sub_segments_expected".to_string(),
                "2".to_string(),
            ),
            (
                "segmentation[0].sub_segment.clear".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn splice_schedule_duration_paths() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_schedule".to_string())])
            .expect("set command");
        doc.apply_sets(&[
            ("splice_schedule.duration".to_string(), "10".to_string()),
            (
                "splice_schedule.utc_splice_time.clear".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn splice_insert_duration_clear() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        doc.apply_sets(&[
            ("splice_insert.duration".to_string(), "90".to_string()),
            (
                "splice_insert.duration.clear".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn delete_target_errors() {
        let mut doc = time_signal_doc();
        let err = doc.delete_target("segmentation[0]").expect_err("missing");
        assert!(err.contains("not found"));
        let err = doc.delete_target("bogus").expect_err("bad target");
        assert!(err.contains("unsupported delete target"));
    }

    #[test]
    fn parse_format_errors() {
        let err = detect_format("garbage").expect_err("unknown");
        assert!(err.contains("unable to detect"));
        let err = Scte35Document::parse(
            "0x1",
            InputFormat::Auto,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect_err("auto error");
        assert!(err.contains("unable to detect"));
        let err = Scte35Document::parse(
            "notbase64",
            InputFormat::Base64,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect_err("base64 error");
        assert!(err.contains("base64 decode error"));
        let err = Scte35Document::parse(
            "0xzz",
            InputFormat::Hex,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect_err("hex error");
        assert!(err.contains("hex decode error"));
        let err = Scte35Document::parse(
            "{",
            InputFormat::Json,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect_err("json error");
        assert!(err.contains("json parse error"));
        let err = Scte35Document::parse(
            "00",
            InputFormat::Hex,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect_err("hex parse error");
        assert!(err.contains("scte35 parse error"));
    }

    #[test]
    fn parse_with_crc_validation_fails_on_corruption() {
        let doc = time_signal_doc();
        let hex = doc.render(OutputFormat::Hex).expect("render hex");
        let bytes = hex::decode(hex).expect("decode");
        parse_message_bytes(&bytes, ParseSettings { validate_crc: true }).expect("crc ok");
        let mut bad = bytes.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xFF;
        let err = parse_message_bytes(&bad, ParseSettings { validate_crc: true }).expect_err("crc");
        assert!(err.contains("parse error") || err.contains("crc"));
    }

    #[test]
    fn parse_message_bytes_crc_false_path() {
        let doc = time_signal_doc();
        let hex = doc.render(OutputFormat::Hex).expect("render hex");
        let bytes = hex::decode(hex).expect("decode");
        let _ = parse_message_bytes(
            &bytes,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect("parse");
    }

    #[test]
    fn parse_message_bytes_errors_on_empty_input() {
        let err = parse_message_bytes(
            &[],
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect_err("parse error");
        assert!(err.contains("scte35 parse error"));
    }

    #[test]
    fn render_output_formats() {
        let doc = time_signal_doc();
        let json = doc.render(OutputFormat::Json).expect("json");
        assert!(json.contains("splice_command"));
        let b64 = doc.render(OutputFormat::Base64).expect("base64");
        assert!(!b64.is_empty());
        let hex = doc.render(OutputFormat::Hex).expect("hex");
        assert!(!hex.is_empty());
    }

    #[test]
    fn parse_input_formats() {
        let doc = time_signal_doc();
        let json = doc.render(OutputFormat::Json).expect("json");
        let parsed = Scte35Document::parse(
            &json,
            InputFormat::Json,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect("parse json");
        let _ = parsed.render(OutputFormat::Json).expect("render");

        let hex = doc.render(OutputFormat::Hex).expect("hex");
        let parsed =
            Scte35Document::parse(&hex, InputFormat::Hex, ParseSettings { validate_crc: true })
                .expect("parse hex");
        let _ = parsed.render(OutputFormat::Json).expect("render");

        let b64 = doc.render(OutputFormat::Base64).expect("base64");
        let parsed = Scte35Document::parse(
            &b64,
            InputFormat::Base64,
            ParseSettings { validate_crc: true },
        )
        .expect("parse base64");
        let _ = parsed.render(OutputFormat::Json).expect("render");
    }

    #[test]
    fn parse_input_auto_detection() {
        let doc = time_signal_doc();
        let json = doc.render(OutputFormat::Json).expect("json");
        let _ = Scte35Document::parse(
            &json,
            InputFormat::Auto,
            ParseSettings {
                validate_crc: false,
            },
        )
        .expect("parse auto json");
        let hex = format!("0x{}", doc.render(OutputFormat::Hex).expect("hex"));
        let _ = Scte35Document::parse(
            &hex,
            InputFormat::Auto,
            ParseSettings { validate_crc: true },
        )
        .expect("parse auto hex");
        let b64 = doc.render(OutputFormat::Base64).expect("base64");
        let _ = Scte35Document::parse(
            &b64,
            InputFormat::Auto,
            ParseSettings { validate_crc: true },
        )
        .expect("parse auto base64");
    }

    #[derive(Deserialize)]
    struct ParserResponse {
        #[serde(rename = "Errors")]
        errors: Vec<String>,
        #[serde(rename = "Success")]
        success: bool,
    }

    fn validate_remote(url: &str, payload: &str) {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(15))
            .build();
        let body = serde_json::to_string(payload).expect("serialize payload");
        let response = agent
            .post(url)
            .set("content-type", "application/json")
            .send_string(&body);
        let response = match response {
            Ok(response) => response,
            Err(ureq::Error::Status(code, response)) => {
                let body = response.into_string().unwrap_or_default();
                panic!("remote parser status {code}: {body}");
            }
            Err(ureq::Error::Transport(err)) => {
                panic!("remote parser transport error: {err}");
            }
        };
        let body = response.into_string().unwrap_or_default();
        let parsed: ParserResponse =
            serde_json::from_str(&body).expect("invalid parser JSON response");
        assert!(
            parsed.success && parsed.errors.is_empty(),
            "remote parser errors: {body}"
        );
    }

    #[test]
    fn validate_remote_parser_hex_and_base64() {
        let doc = time_signal_doc();
        let hex = doc.render(OutputFormat::Hex).expect("hex");
        validate_remote(
            "https://scte10435parser.middleman.tv/parse-scte35-from-hex?constrain-to-35-standard=255",
            &hex,
        );
        let b64 = doc.render(OutputFormat::Base64).expect("base64");
        validate_remote(
            "https://scte10435parser.middleman.tv/parse-scte35-from-base64?constrain-to-35-standard=255",
            &b64,
        );
    }

    #[test]
    fn supported_paths_returns_data() {
        let paths = supported_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn validate_patch_value_indexed_paths() {
        validate_patch_value("segmentation[0].event_id", "1").expect("ok");
        validate_patch_value("segmentation[0].segments_expected", "2").expect("ok");
        validate_patch_value("avail[0].provider_id", "0x0102").expect("ok");
        validate_patch_value("avail[0].identifier", "33").expect("ok");
        validate_patch_value("dtmf[0].preroll", "2").expect("ok");
        validate_patch_value("dtmf[0].chars", "0x3132").expect("ok");
        validate_patch_value("dtmf[0].identifier", "44").expect("ok");
        validate_patch_value("time[0].tai_seconds", "0x01").expect("ok");
        validate_patch_value("time[0].tai_ns", "0x02").expect("ok");
        validate_patch_value("time[0].utc_offset", "0x03").expect("ok");
        validate_patch_value("time[0].identifier", "55").expect("ok");
        validate_patch_value("audio[0].components", "0x01").expect("ok");
        validate_patch_value("audio[0].identifier", "66").expect("ok");
        validate_patch_value("unknown[0].tag", "1").expect("ok");
        validate_patch_value("unknown[0].data", "0x00").expect("ok");
        let err = validate_patch_value("splice_insert.component[0].bogus", "1")
            .expect_err("bad component spec");
        assert!(err.contains("unsupported path"));
    }

    #[test]
    fn patch_op_parses_indexed_component_fields() {
        let op = PatchOp::new("splice_insert.component[0].tag", "1").expect("op");
        match op.path {
            PatchPath::SpliceInsertComponent { index, field } => {
                assert_eq!(index, 0);
                assert!(matches!(field, SpliceInsertComponentField::Tag));
            }
            _ => panic!("unexpected path"),
        }
    }

    #[test]
    fn patch_op_parses_schedule_component_fields() {
        let op = PatchOp::new("splice_schedule.component[0].duration_flag", "true").expect("op");
        match op.path {
            PatchPath::SpliceScheduleComponent { index, field } => {
                assert_eq!(index, 0);
                assert!(matches!(field, SpliceScheduleComponentField::DurationFlag));
            }
            _ => panic!("unexpected path"),
        }
    }

    #[test]
    fn apply_ops_runs_all() {
        let mut doc = time_signal_doc();
        let ops = vec![
            PatchOp::new("table_id", "252").unwrap(),
            PatchOp::new("cw_index", "2").unwrap(),
        ];
        doc.apply_ops(&ops).expect("apply");
    }

    #[test]
    fn apply_segmentation_delivery_not_restricted_true() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            (
                "segmentation[0].delivery_not_restricted".to_string(),
                "false".to_string(),
            ),
            (
                "segmentation[0].web_delivery_allowed".to_string(),
                "true".to_string(),
            ),
            (
                "segmentation[0].delivery_not_restricted".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn parse_patch_path_all_indexed_fields() {
        let _ = parse_patch_path("avail[0].identifier").expect("avail");
        let _ = parse_patch_path("dtmf[0].identifier").expect("dtmf");
        let _ = parse_patch_path("time[0].identifier").expect("time");
        let _ = parse_patch_path("audio[0].identifier").expect("audio");
        let _ = parse_patch_path("unknown[0].data").expect("unknown");
    }

    #[test]
    fn apply_time_identifier_paths() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            ("dtmf.identifier".to_string(), "1122".to_string()),
            ("time.identifier".to_string(), "3344".to_string()),
            ("audio.identifier".to_string(), "5566".to_string()),
            ("unknown.data".to_string(), "0x0102".to_string()),
        ])
        .expect("apply");
    }

    #[test]
    fn apply_descriptor_fields_more() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            ("dtmf.preroll".to_string(), "3".to_string()),
            ("dtmf.chars".to_string(), "0x3132".to_string()),
            ("time.tai_seconds".to_string(), "0x0102".to_string()),
            ("time.tai_ns".to_string(), "0x0304".to_string()),
            ("time.utc_offset".to_string(), "0x05".to_string()),
            ("audio.components".to_string(), "0x0102".to_string()),
            ("unknown.tag".to_string(), "7".to_string()),
        ])
        .expect("apply");
    }

    #[test]
    fn splice_schedule_component_duration_flag_toggle() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_schedule".to_string())])
            .expect("set command");
        let schedule = splice_schedule_mut(&mut doc.section).expect("schedule");
        schedule.component_list.push(scte35::ComponentSplice {
            component_tag: 1,
            reserved: 0,
            splice_mode_indicator: 1,
            duration_flag: 0,
            splice_duration: None,
            utc_splice_time: Some(10),
        });
        schedule.num_splice = 1;
        doc.apply_sets(&[
            (
                "splice_schedule.component[0].duration".to_string(),
                "10".to_string(),
            ),
            (
                "splice_schedule.component[0].utc_splice_time".to_string(),
                "20".to_string(),
            ),
            (
                "splice_schedule.component[0].duration_flag".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn delete_target_splice_components() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        doc.apply_sets(&[(
            "splice_insert.component.add".to_string(),
            "tag=1,pts=90000".to_string(),
        )])
        .expect("add");
        doc.delete_target("splice_insert.component[0]")
            .expect("delete insert component");

        doc.apply_sets(&[("splice_command".to_string(), "splice_schedule".to_string())])
            .expect("set schedule");
        let schedule = splice_schedule_mut(&mut doc.section).expect("schedule");
        schedule.component_list.push(scte35::ComponentSplice {
            component_tag: 1,
            reserved: 0,
            splice_mode_indicator: 1,
            duration_flag: 0,
            splice_duration: None,
            utc_splice_time: Some(10),
        });
        schedule.num_splice = 1;
        doc.delete_target("splice_schedule.component[0]")
            .expect("delete schedule component");

        let err = doc
            .delete_target("splice_schedule.component[0]")
            .expect_err("not found");
        assert!(err.contains("splice_schedule.component[0] not found"));
    }

    #[test]
    fn parse_patch_path_for_indexed_variants() {
        let path = parse_patch_path("dtmf[1].identifier").expect("path");
        match path {
            PatchPath::Descriptor { kind, index, field } => {
                assert!(matches!(kind, DescriptorKind::Dtmf));
                assert_eq!(index, 1);
                assert!(matches!(field, DescriptorField::DtmfIdentifier));
            }
            _ => panic!("unexpected path"),
        }
    }

    #[test]
    fn parse_indexed_errors() {
        assert!(parse_indexed("segmentation[1]", "segmentation").is_none());
        assert!(parse_index_only("segmentation[1].event_id", "segmentation").is_none());
        assert!(parse_indexed("segmentation[no].event_id", "segmentation").is_none());
    }

    #[test]
    fn splice_command_variants() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_null".to_string())])
            .expect("splice null");
        doc.apply_sets(&[(
            "splice_command".to_string(),
            "bandwidth_reservation".to_string(),
        )])
        .expect("bandwidth");
        doc.apply_sets(&[("splice_command".to_string(), "private_command".to_string())])
            .expect("private");
    }

    #[test]
    fn splice_command_time_signal_rebuild() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "time_signal".to_string())])
            .expect("time signal");
    }

    #[test]
    fn splice_schedule_component_add_reaches_apply_op_error() {
        let mut doc = time_signal_doc();
        let op = PatchOp {
            path: PatchPath::SpliceScheduleComponentAdd,
            value: "tag=1".to_string(),
        };
        let err = doc.apply_ops(&[op]).expect_err("expected error");
        assert!(err.contains("not supported"));
    }

    #[test]
    fn splice_insert_program_splice_flag_clears_components() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        doc.apply_sets(&[(
            "splice_insert.component.add".to_string(),
            "tag=1,pts=90000".to_string(),
        )])
        .expect("add component");
        doc.apply_sets(&[(
            "splice_insert.program_splice".to_string(),
            "true".to_string(),
        )])
        .expect("program splice");
    }

    #[test]
    fn splice_insert_immediate_flag_clears_time() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_insert".to_string())])
            .expect("set command");
        doc.apply_sets(&[
            (
                "splice_insert.splice_time.pts_time".to_string(),
                "90000".to_string(),
            ),
            (
                "splice_insert.splice_time.immediate".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn splice_schedule_component_out_of_range() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("splice_command".to_string(), "splice_schedule".to_string())])
            .expect("set command");
        let schedule = splice_schedule_mut(&mut doc.section).expect("schedule");
        schedule.component_list.push(scte35::ComponentSplice {
            component_tag: 1,
            reserved: 0,
            splice_mode_indicator: 1,
            duration_flag: 0,
            splice_duration: None,
            utc_splice_time: Some(10),
        });
        schedule.num_splice = 1;
        let err = doc
            .apply_sets(&[(
                "splice_schedule.component[1].tag".to_string(),
                "2".to_string(),
            )])
            .expect_err("out of range");
        assert!(err.contains("splice_schedule.component[1] not found"));
    }

    #[test]
    fn splice_schedule_mut_wrong_command_errors() {
        let mut doc = time_signal_doc();
        let err = splice_schedule_mut(&mut doc.section).expect_err("wrong command");
        assert!(err.contains("splice_schedule paths are only supported"));
    }

    #[test]
    fn splice_insert_mut_wrong_command_errors() {
        let mut doc = time_signal_doc();
        let err = splice_insert_mut(&mut doc.section).expect_err("wrong command");
        assert!(err.contains("splice_insert paths are only supported"));
    }

    #[test]
    fn apply_segmentation_paths_non_indexed() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[
            ("segmentation.event_id".to_string(), "1".to_string()),
            ("segmentation.cancel".to_string(), "false".to_string()),
            ("segmentation.program".to_string(), "true".to_string()),
            ("segmentation.duration".to_string(), "90000".to_string()),
            (
                "segmentation.duration.clear".to_string(),
                "true".to_string(),
            ),
            (
                "segmentation.delivery_not_restricted".to_string(),
                "true".to_string(),
            ),
            (
                "segmentation.web_delivery_allowed".to_string(),
                "true".to_string(),
            ),
            (
                "segmentation.no_regional_blackout".to_string(),
                "true".to_string(),
            ),
            (
                "segmentation.archive_allowed".to_string(),
                "false".to_string(),
            ),
            (
                "segmentation.device_restrictions".to_string(),
                "3".to_string(),
            ),
            ("segmentation.upid_type".to_string(), "0".to_string()),
            ("segmentation.upid".to_string(), "0x4142".to_string()),
            ("segmentation.type_id".to_string(), "48".to_string()),
            ("segmentation.segment_num".to_string(), "1".to_string()),
            (
                "segmentation.segments_expected".to_string(),
                "2".to_string(),
            ),
            ("segmentation.sub_segment_num".to_string(), "1".to_string()),
            (
                "segmentation.sub_segments_expected".to_string(),
                "2".to_string(),
            ),
            (
                "segmentation.sub_segment.clear".to_string(),
                "true".to_string(),
            ),
        ])
        .expect("apply");
    }

    #[test]
    fn apply_avail_identifier_non_indexed() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("avail.identifier".to_string(), "1129531753".to_string())])
            .expect("apply");
    }

    #[test]
    fn descriptor_mut_access_errors() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("avail[0].provider_id".to_string(), "0x01".to_string())])
            .expect("apply");
        let desc = dtmf_mut(&mut doc.section).expect("dtmf created");
        assert_eq!(desc.identifier, 0x4355_4549);
    }

    #[test]
    fn descriptor_at_errors() {
        let mut doc = time_signal_doc();
        let err = descriptor_at(&mut doc.section, DescriptorKind::Avail, 0).expect_err("missing");
        assert!(err.contains("descriptor[0] not found"));
    }

    #[test]
    fn apply_descriptor_field_errors() {
        let mut doc = time_signal_doc();
        let err = apply_avail_field(&mut doc, 0, DescriptorField::DtmfChars, "0x01")
            .expect_err("unsupported");
        assert!(err.contains("unsupported avail field"));
        let err = apply_dtmf_field(&mut doc, 0, DescriptorField::AvailProviderId, "0x01")
            .expect_err("unsupported");
        assert!(err.contains("unsupported dtmf field"));
        let err = apply_time_field(&mut doc, 0, DescriptorField::AudioComponents, "0x01")
            .expect_err("unsupported");
        assert!(err.contains("unsupported time field"));
        let err = apply_audio_field(&mut doc, 0, DescriptorField::UnknownTag, "1")
            .expect_err("unsupported");
        assert!(err.contains("unsupported audio field"));
        let err = apply_unknown_field(&mut doc, 0, DescriptorField::AudioComponents, "0x01")
            .expect_err("unsupported");
        assert!(err.contains("unsupported unknown field"));
    }

    #[test]
    fn descriptor_type_mismatch_unreachable() {
        let mut doc = time_signal_doc();
        doc.apply_sets(&[("segmentation.event_id".to_string(), "1".to_string())])
            .expect("apply");
        let seg = segmentation_mut(&mut doc.section).expect("seg");
        assert_eq!(seg.segmentation_event_id >> 24, 1);
    }

    #[test]
    fn patch_op_rejects_unknown_path() {
        let err = PatchOp::new("bogus", "1").expect_err("bad path");
        assert!(err.contains("unsupported path"));
    }
}

fn parse_u8(path: &str, value: &str) -> Result<u8, String> {
    value
        .trim()
        .parse::<u8>()
        .map_err(|_| format!("invalid value for {path}: expected u8"))
}

fn parse_u16(path: &str, value: &str) -> Result<u16, String> {
    value
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("invalid value for {path}: expected u16"))
}

fn parse_u64(path: &str, value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {path}: expected u64"))
}

fn parse_u32(path: &str, value: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("invalid value for {path}: expected u32"))
}

fn parse_u32_be(path: &str, value: &str) -> Result<u32, String> {
    let parsed = parse_u32(path, value)?;
    Ok(parsed << 24)
}

fn parse_bool(path: &str, value: &str) -> Result<bool, String> {
    value
        .trim()
        .parse::<bool>()
        .map_err(|_| format!("invalid value for {path}: expected true/false"))
}

fn parse_bytes(value: &str) -> Result<Vec<u8>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "0x" {
        return Ok(Vec::new());
    }
    if trimmed.starts_with("0x") {
        return hex::decode(trimmed.trim_start_matches("0x"))
            .map_err(|err| format!("invalid hex bytes: {err}"));
    }
    if let Ok(bytes) = hex::decode(trimmed) {
        return Ok(bytes);
    }
    base64::engine::general_purpose::STANDARD
        .decode(trimmed.as_bytes())
        .map_err(|err| format!("invalid base64 bytes: {err}"))
}

fn parse_splice_insert_component(value: &str) -> Result<scte35::SpliceInsertComponent, String> {
    let mut component_tag = None;
    let mut pts_time = None;
    for part in value.split(',') {
        let (key, val) = part
            .split_once('=')
            .ok_or_else(|| format!("invalid component spec '{value}'"))?;
        match key.trim() {
            "tag" => component_tag = Some(parse_u8("component.tag", val)?),
            "pts" => pts_time = Some(parse_u64("component.pts", val)? & 0x1_FFFF_FFFF),
            "immediate" => {
                let immediate = parse_bool("component.immediate", val)?;
                if immediate {
                    pts_time = None;
                }
            }
            _ => return Err(format!("unknown component field '{key}'")),
        }
    }
    let tag = component_tag.ok_or_else(|| "component requires tag".to_string())?;
    let splice_time = pts_time.map(|pts| scte35::SpliceTime {
        time_specified_flag: 1,
        pts_time: Some(pts),
    });
    Ok(scte35::SpliceInsertComponent {
        component_tag: tag,
        splice_time,
    })
}

fn parse_message_bytes(bytes: &[u8], settings: ParseSettings) -> Result<Scte35Document, String> {
    let section =
        parse_splice_info_section(bytes).map_err(|err| format!("scte35 parse error: {err}"))?;
    if settings.validate_crc {
        let crc_ok =
            scte35::validate_scte35_crc(bytes).map_err(|err| format!("crc check failed: {err}"))?;
        if !crc_ok {
            return Err("crc validation failed".into());
        }
    }
    Ok(Scte35Document { section })
}

fn detect_format(input: &str) -> Result<InputFormat, String> {
    let trimmed = input.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            return Ok(InputFormat::Json);
        }
    }

    let hex_candidate = trimmed.strip_prefix("0x").unwrap_or(trimmed).trim();
    if is_hex_candidate(hex_candidate) {
        return Ok(InputFormat::Hex);
    }

    // fall back to base64 if decoding works
    if BASE64_STANDARD.decode(trimmed).is_ok() {
        return Ok(InputFormat::Base64);
    }

    Err("unable to detect input format".into())
}

fn is_hex_candidate(input: &str) -> bool {
    let len = input.len();
    if len == 0 || (len % 2) != 0 {
        return false;
    }
    input
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
}
