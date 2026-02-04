use crate::core::{OutputFormat, ParseSettings, Scte35Document, supported_paths_meta};
use crate::interactive;
use crate::io::{InputFormat, InputSpec, OutputTarget, load_input, write_output};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "scte35-editor",
    version,
    about = "Create and edit SCTE-35 triggers"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(&self) -> Result<(), String> {
        match &self.command {
            Command::Show(args) => run_show(args),
            Command::Validate(args) => run_validate(args),
            Command::Edit(args) => run_edit(args),
            Command::New(args) => run_new(args),
            Command::ListPaths => {
                print_paths();
                Ok(())
            }
            Command::Delete(args) => run_delete(args),
            Command::Interactive(args) => run_interactive(args),
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Parse input and render output
    #[command(alias = "s")]
    Show(ShowArgs),
    /// Parse input and validate
    #[command(alias = "v")]
    Validate(ShowArgs),
    /// Parse input and apply modifications
    #[command(alias = "e")]
    Edit(EditArgs),
    /// Create a new splice info section from arguments
    #[command(alias = "n")]
    New(NewArgs),
    /// List supported --set paths
    ListPaths,
    /// Delete an indexed component or descriptor
    #[command(alias = "d")]
    Delete(DeleteArgs),
    /// Launch interactive editor
    #[command(alias = "i")]
    Interactive(InteractiveArgs),
}

#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Inline input string (positional)
    #[arg(value_name = "INPUT", conflicts_with = "file")]
    input: Option<String>,
    /// Inline input string (flag)
    #[arg(long = "input", value_name = "INPUT", conflicts_with = "file")]
    input_flag: Option<String>,
    /// Read input from file
    #[arg(long, value_name = "PATH", conflicts_with_all = ["input", "input_flag"])]
    file: Option<PathBuf>,
    /// Input format
    #[arg(long, value_enum, default_value_t = InputFormatCli::Auto)]
    input_format: InputFormatCli,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormatCli::Json)]
    output_format: OutputFormatCli,
    /// Write output to file instead of stdout
    #[arg(long, value_name = "PATH")]
    output_file: Option<PathBuf>,
    /// Skip CRC validation if supported by the parser
    #[arg(long, default_value_t = false)]
    no_crc: bool,
    /// Fail on unknown JSON fields
    #[arg(long, default_value_t = false)]
    strict: bool,
}

#[derive(Parser, Debug)]
pub struct EditArgs {
    #[arg(value_name = "INPUT", conflicts_with = "file")]
    input: Option<String>,
    #[arg(long = "input", value_name = "INPUT", conflicts_with = "file")]
    input_flag: Option<String>,
    #[arg(long, value_name = "PATH", conflicts_with_all = ["input", "input_flag"])]
    file: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = InputFormatCli::Auto)]
    input_format: InputFormatCli,
    #[arg(long, value_enum, default_value_t = OutputFormatCli::Json)]
    output_format: OutputFormatCli,
    #[arg(long, value_name = "PATH")]
    output_file: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    no_crc: bool,
    /// Fail on unknown JSON fields
    #[arg(long, default_value_t = false)]
    strict: bool,
    /// Apply a field update in the form path=value (repeatable)
    #[arg(long = "set", value_name = "PATH=VALUE")]
    set: Vec<String>,
    /// List supported --set paths and exit
    #[arg(long, default_value_t = false)]
    list_paths: bool,
}

#[derive(Parser, Debug)]
pub struct NewArgs {
    #[arg(long, value_enum, default_value_t = OutputFormatCli::Json)]
    output_format: OutputFormatCli,
    #[arg(long, value_name = "PATH")]
    output_file: Option<PathBuf>,
    /// Start from a template (optional)
    #[arg(value_name = "TEMPLATE")]
    template: String,
    /// Apply a field update in the form path=value (repeatable)
    #[arg(long = "set", value_name = "PATH=VALUE")]
    set: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct InteractiveArgs {
    #[arg(value_name = "INPUT", conflicts_with = "file")]
    input: Option<String>,
    #[arg(long = "input", value_name = "INPUT", conflicts_with = "file")]
    input_flag: Option<String>,
    #[arg(long, value_name = "PATH", conflicts_with_all = ["input", "input_flag"])]
    file: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = InputFormatCli::Auto)]
    input_format: InputFormatCli,
    /// Output format when writing
    #[arg(long, value_enum, default_value_t = OutputFormatCli::Json)]
    output_format: OutputFormatCli,
    /// Write output to file instead of stdout
    #[arg(long, value_name = "PATH")]
    output_file: Option<PathBuf>,
    /// Skip CRC validation if supported by the parser
    #[arg(long, default_value_t = false)]
    no_crc: bool,
    /// Fail on unknown JSON fields
    #[arg(long, default_value_t = false)]
    strict: bool,
}

#[derive(Parser, Debug)]
pub struct DeleteArgs {
    /// Inline input string (positional)
    #[arg(value_name = "INPUT", conflicts_with = "file")]
    input: Option<String>,
    /// Inline input string (flag)
    #[arg(long = "input", value_name = "INPUT", conflicts_with = "file")]
    input_flag: Option<String>,
    /// Read input from file
    #[arg(long, value_name = "PATH", conflicts_with_all = ["input", "input_flag"])]
    file: Option<PathBuf>,
    /// Input format
    #[arg(long, value_enum, default_value_t = InputFormatCli::Auto)]
    input_format: InputFormatCli,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormatCli::Json)]
    output_format: OutputFormatCli,
    /// Write output to file instead of stdout
    #[arg(long, value_name = "PATH")]
    output_file: Option<PathBuf>,
    /// Skip CRC validation if supported by the parser
    #[arg(long, default_value_t = false)]
    no_crc: bool,
    /// Fail on unknown JSON fields
    #[arg(long, default_value_t = false)]
    strict: bool,
    /// Delete target, e.g. segmentation[0], avail[1], splice_insert.component[2]
    #[arg(long, value_name = "TARGET")]
    target: String,
    /// List supported delete targets and exit
    #[arg(long, default_value_t = false)]
    list_targets: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum InputFormatCli {
    Auto,
    Json,
    Base64,
    Hex,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormatCli {
    Json,
    Base64,
    Hex,
}

fn run_show(args: &ShowArgs) -> Result<(), String> {
    let (input_spec, input_format) = parse_input_spec(
        args.input_flag.as_deref().or(args.input.as_deref()),
        args.file.as_deref(),
        args.input_format,
    )?;
    let input = load_input(&input_spec)?;
    let document = Scte35Document::parse(
        &input,
        input_format,
        ParseSettings {
            validate_crc: !args.no_crc,
            strict: args.strict,
        },
    )?;
    let output = render_output(&document, args.output_format)?;
    write_output(
        OutputTarget::from_path(args.output_file.as_deref()),
        &output,
    )?;
    Ok(())
}

fn run_validate(args: &ShowArgs) -> Result<(), String> {
    let (input_spec, input_format) = parse_input_spec(
        args.input_flag.as_deref().or(args.input.as_deref()),
        args.file.as_deref(),
        args.input_format,
    )?;
    let input = load_input(&input_spec)?;
    let _ = Scte35Document::parse(
        &input,
        input_format,
        ParseSettings {
            validate_crc: !args.no_crc,
            strict: args.strict,
        },
    )?;
    let output = match args.output_format {
        OutputFormatCli::Json => serde_json::to_string_pretty(&serde_json::json!({
            "valid": true
        }))
        .map_err(|err| format!("failed to serialize output: {err}"))?,
        OutputFormatCli::Hex | OutputFormatCli::Base64 => "valid".to_string(),
    };
    write_output(
        OutputTarget::from_path(args.output_file.as_deref()),
        &output,
    )?;
    Ok(())
}

fn run_edit(args: &EditArgs) -> Result<(), String> {
    if args.list_paths {
        print_paths();
        return Ok(());
    }
    let (input_spec, input_format) = parse_input_spec(
        args.input_flag.as_deref().or(args.input.as_deref()),
        args.file.as_deref(),
        args.input_format,
    )?;
    let input = load_input(&input_spec)?;
    let mut document = Scte35Document::parse(
        &input,
        input_format,
        ParseSettings {
            validate_crc: !args.no_crc,
            strict: args.strict,
        },
    )?;

    if args.set.is_empty() {
        return Err("no changes provided; use --set path=value".into());
    }

    let changes = parse_sets(&args.set)?;
    document.apply_sets(&changes)?;
    let output = render_output(&document, args.output_format)?;

    write_output(
        OutputTarget::from_path(args.output_file.as_deref()),
        &output,
    )?;
    Ok(())
}

fn run_new(args: &NewArgs) -> Result<(), String> {
    let document = match args.template.as_str() {
        "time-signal" => Scte35Document::new_default_time_signal()?,
        "splice-null" => Scte35Document::new_default_splice_null()?,
        "splice-insert" => Scte35Document::new_default_splice_insert()?,
        "splice-schedule" => Scte35Document::new_default_splice_schedule()?,
        other => return Err(format!("unsupported template '{other}'")),
    };
    let mut document = document;
    if args.set.is_empty() {
        return Err("new requires --set path=value".into());
    }
    let changes = parse_sets(&args.set)?;
    document.apply_sets(&changes)?;
    let output = render_output(&document, args.output_format)?;
    write_output(
        OutputTarget::from_path(args.output_file.as_deref()),
        &output,
    )?;
    Ok(())
}

fn render_output(
    document: &Scte35Document,
    output_format: OutputFormatCli,
) -> Result<String, String> {
    match output_format {
        OutputFormatCli::Json => {
            let json_output = document.render(OutputFormat::Json)?;
            let json_value: serde_json::Value = serde_json::from_str(&json_output)
                .map_err(|err| format!("failed to parse rendered json output: {err}"))?;
            let hex_output = document.render(OutputFormat::Hex)?;
            let base64_output = document.render(OutputFormat::Base64)?;
            serde_json::to_string_pretty(&serde_json::json!({
                "json": json_value,
                "hex": hex_output,
                "base64": base64_output
            }))
            .map_err(|err| format!("failed to serialize output: {err}"))
        }
        OutputFormatCli::Hex => document.render(OutputFormat::Hex),
        OutputFormatCli::Base64 => document.render(OutputFormat::Base64),
    }
}

fn parse_input_spec(
    input: Option<&str>,
    file: Option<&Path>,
    format: InputFormatCli,
) -> Result<(InputSpec, InputFormat), String> {
    let input_spec = if let Some(value) = input {
        InputSpec::Inline(value.to_string())
    } else if let Some(path) = file {
        InputSpec::File(path.to_path_buf())
    } else {
        return Err("expected INPUT or --file".into());
    };
    Ok((input_spec, format.into()))
}

fn parse_sets(values: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut parsed = Vec::with_capacity(values.len());
    for value in values {
        let (path, rhs) = value
            .split_once('=')
            .ok_or_else(|| format!("invalid --set '{value}', expected path=value"))?;
        if path.trim().is_empty() {
            return Err(format!("invalid --set '{value}', path is empty"));
        }
        parsed.push((path.trim().to_string(), rhs.trim().to_string()));
    }
    Ok(parsed)
}

fn run_delete(args: &DeleteArgs) -> Result<(), String> {
    if args.list_targets {
        print_delete_targets();
        return Ok(());
    }
    let (input_spec, input_format) = parse_input_spec(
        args.input_flag.as_deref().or(args.input.as_deref()),
        args.file.as_deref(),
        args.input_format,
    )?;
    let input = load_input(&input_spec)?;
    let mut document = Scte35Document::parse(
        &input,
        input_format,
        ParseSettings {
            validate_crc: !args.no_crc,
            strict: args.strict,
        },
    )?;
    document.delete_target(&args.target)?;
    let output = render_output(&document, args.output_format)?;
    write_output(
        OutputTarget::from_path(args.output_file.as_deref()),
        &output,
    )?;
    Ok(())
}

fn run_interactive(args: &InteractiveArgs) -> Result<(), String> {
    let args = interactive::InteractiveArgs {
        input: args.input_flag.clone().or_else(|| args.input.clone()),
        file: args.file.clone(),
        input_format: args.input_format.into(),
        output_format: args.output_format.into(),
        output_file: args.output_file.clone(),
        no_crc: args.no_crc,
        strict: args.strict,
    };
    interactive::run(&args)
}

fn print_delete_targets() {
    let targets = [
        "segmentation[i]",
        "avail[i]",
        "dtmf[i]",
        "time[i]",
        "audio[i]",
        "unknown[i]",
        "splice_insert.component[i]",
        "splice_schedule.component[i]",
    ];
    for target in targets {
        println!("{target}");
    }
}

fn print_paths() {
    for meta in supported_paths_meta() {
        println!(
            "{path}\t{value_type:?}\t{constraints}\t{example}",
            path = meta.path,
            value_type = meta.value_type,
            constraints = meta.constraints,
            example = meta.example
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::Path;

    #[test]
    fn parse_sets_parses_values() {
        let values = vec!["table_id=252".to_string(), "tier=4095".to_string()];
        let parsed = parse_sets(&values).expect("parse_sets failed");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "table_id");
        assert_eq!(parsed[0].1, "252");
    }

    #[test]
    fn parse_sets_rejects_missing_equals() {
        let values = vec!["table_id".to_string()];
        let err = parse_sets(&values).expect_err("expected error");
        assert!(err.contains("expected path=value"));
    }

    #[test]
    fn print_paths_does_not_panic() {
        print_paths();
    }

    #[test]
    fn print_delete_targets_does_not_panic() {
        print_delete_targets();
    }

    #[test]
    fn run_delete_list_targets() {
        let args = DeleteArgs {
            input: None,
            input_flag: None,
            file: None,
            input_format: super::InputFormatCli::Auto,
            output_format: super::OutputFormatCli::Json,
            output_file: None,
            no_crc: false,
            strict: false,
            target: "segmentation[0]".to_string(),
            list_targets: true,
        };
        let result = run_delete(&args);
        assert!(result.is_ok());
    }

    fn valid_json_input() -> String {
        Scte35Document::new_default_time_signal()
            .expect("doc")
            .render(OutputFormat::Json)
            .expect("render")
    }

    fn valid_json_with_segmentation() -> String {
        let mut doc = Scte35Document::new_default_time_signal().expect("doc");
        doc.apply_sets(&[("segmentation[0].event_id".to_string(), "1".to_string())])
            .expect("apply");
        doc.render(OutputFormat::Json).expect("render")
    }

    fn valid_hex_input() -> String {
        Scte35Document::new_default_time_signal()
            .expect("doc")
            .render(OutputFormat::Hex)
            .expect("render")
    }

    fn valid_base64_input() -> String {
        Scte35Document::new_default_time_signal()
            .expect("doc")
            .render(OutputFormat::Base64)
            .expect("render")
    }

    #[test]
    fn cli_run_show_command() {
        let input = valid_json_input();
        let cli = Cli::parse_from([
            "scte35-editor",
            "show",
            &input,
            "--input-format",
            "json",
            "--output-format",
            "json",
        ]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn cli_run_validate_command() {
        let input = valid_json_input();
        let cli = Cli::parse_from([
            "scte35-editor",
            "validate",
            &input,
            "--input-format",
            "json",
        ]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn cli_run_edit_command() {
        let input = valid_json_input();
        let cli = Cli::parse_from([
            "scte35-editor",
            "edit",
            &input,
            "--input-format",
            "json",
            "--set",
            "table_id=252",
            "--output-format",
            "json",
        ]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn cli_run_new_command() {
        let cli = Cli::parse_from([
            "scte35-editor",
            "new",
            "time-signal",
            "--set",
            "table_id=252",
            "--output-format",
            "json",
        ]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn cli_run_delete_command() {
        let input = valid_json_with_segmentation();
        let cli = Cli::parse_from([
            "scte35-editor",
            "delete",
            &input,
            "--input-format",
            "json",
            "--target",
            "segmentation[0]",
            "--output-format",
            "json",
        ]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn cli_run_list_paths_command() {
        let cli = Cli::parse_from(["scte35-editor", "list-paths"]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn parse_input_spec_errors_without_input() {
        let err = parse_input_spec(None, None, InputFormatCli::Auto).expect_err("err");
        assert!(err.contains("expected INPUT"));
    }

    #[test]
    fn parse_input_spec_variants() {
        let (spec, format) =
            parse_input_spec(Some("{}"), None, InputFormatCli::Json).expect("spec");
        assert!(matches!(spec, InputSpec::Inline(_)));
        assert!(matches!(format, InputFormat::Json));

        let path = Path::new("target/input.json");
        let (spec, format) = parse_input_spec(None, Some(path), InputFormatCli::Hex).expect("spec");
        assert!(matches!(spec, InputSpec::File(_)));
        assert!(matches!(format, InputFormat::Hex));
    }

    #[test]
    fn parse_sets_rejects_empty_path() {
        let values = vec!["=1".to_string()];
        let err = parse_sets(&values).expect_err("expected error");
        assert!(err.contains("path is empty"));
    }

    #[test]
    fn run_show_missing_input_errors() {
        let args = ShowArgs {
            input: None,
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Auto,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: false,
            strict: false,
        };
        let err = run_show(&args).expect_err("expected error");
        assert!(err.contains("expected INPUT"));
    }

    #[test]
    fn run_show_with_inline_input() {
        let args = ShowArgs {
            input: Some(valid_json_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Json,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
        };
        let result = run_show(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_show_hex_input() {
        let args = ShowArgs {
            input: Some(valid_hex_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Hex,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
        };
        let result = run_show(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_show_base64_input() {
        let args = ShowArgs {
            input: Some(valid_base64_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Base64,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
        };
        let result = run_show(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_validate_with_inline_input() {
        let args = ShowArgs {
            input: Some(valid_json_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Json,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
        };
        let result = run_validate(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_edit_list_paths() {
        let args = EditArgs {
            input: None,
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Auto,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: false,
            strict: false,
            set: Vec::new(),
            list_paths: true,
        };
        let result = run_edit(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_edit_requires_set() {
        let args = EditArgs {
            input: Some(valid_json_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Json,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
            set: Vec::new(),
            list_paths: false,
        };
        let err = run_edit(&args).expect_err("expected error");
        assert!(err.contains("no changes provided"));
    }

    #[test]
    fn run_edit_applies_change() {
        let args = EditArgs {
            input: Some(valid_json_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Json,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
            set: vec!["table_id=252".to_string()],
            list_paths: false,
        };
        let result = run_edit(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_new_template_error() {
        let args = NewArgs {
            output_format: OutputFormatCli::Json,
            output_file: None,
            template: "unknown".to_string(),
            set: Vec::new(),
        };
        let err = run_new(&args).expect_err("expected error");
        assert!(err.contains("unsupported template"));
    }

    #[test]
    fn run_new_requires_set() {
        let args = NewArgs {
            output_format: OutputFormatCli::Json,
            output_file: None,
            template: "time-signal".to_string(),
            set: Vec::new(),
        };
        let err = run_new(&args).expect_err("expected error");
        assert!(err.contains("requires --set"));
    }

    #[test]
    fn run_new_with_set() {
        let args = NewArgs {
            output_format: OutputFormatCli::Json,
            output_file: None,
            template: "time-signal".to_string(),
            set: vec!["table_id=252".to_string()],
        };
        let result = run_new(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_new_with_file_output() {
        let dir = env::temp_dir();
        let output_path = dir.join("scte35-new.json");
        let args = NewArgs {
            output_format: OutputFormatCli::Json,
            output_file: Some(output_path.clone()),
            template: "splice-null".to_string(),
            set: vec!["table_id=252".to_string()],
        };
        let result = run_new(&args);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn run_delete_missing_input_errors() {
        let args = DeleteArgs {
            input: None,
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Auto,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: false,
            strict: false,
            target: "segmentation[0]".to_string(),
            list_targets: false,
        };
        let err = run_delete(&args).expect_err("expected error");
        assert!(err.contains("expected INPUT"));
    }

    #[test]
    fn run_delete_with_file_input() {
        let dir = env::temp_dir();
        let input_path = dir.join("scte35-delete.json");
        let output_path = dir.join("scte35-delete-out.json");
        fs::write(&input_path, valid_json_with_segmentation()).expect("write input");

        let args = DeleteArgs {
            input: None,
            input_flag: None,
            file: Some(input_path.clone()),
            input_format: InputFormatCli::Json,
            output_format: OutputFormatCli::Json,
            output_file: Some(output_path.clone()),
            no_crc: true,
            strict: false,
            target: "segmentation[0]".to_string(),
            list_targets: false,
        };
        let result = run_delete(&args);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn run_delete_splice_insert_component_not_found() {
        let args = DeleteArgs {
            input: Some(valid_json_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Json,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
            target: "splice_insert.component[0]".to_string(),
            list_targets: false,
        };
        let err = run_delete(&args).expect_err("expected error");
        assert!(err.contains("splice_insert paths are only supported"));
    }

    #[test]
    fn run_delete_splice_schedule_component_not_found() {
        let args = DeleteArgs {
            input: Some(valid_json_input()),
            input_flag: None,
            file: None,
            input_format: InputFormatCli::Json,
            output_format: OutputFormatCli::Json,
            output_file: None,
            no_crc: true,
            strict: false,
            target: "splice_schedule.component[0]".to_string(),
            list_targets: false,
        };
        let err = run_delete(&args).expect_err("expected error");
        assert!(err.contains("splice_schedule paths are only supported"));
    }

    #[test]
    fn run_with_args_interactive_unimplemented() {
        let cli = Cli::parse_from(["scte35-editor", "interactive", "{}"]);
        let result = cli.run();
        assert!(result.is_err());
    }

    #[test]
    fn cli_run_list_paths() {
        let cli = Cli::parse_from(["scte35-editor", "list-paths"]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn cli_run_delete_list_targets() {
        let cli = Cli::parse_from([
            "scte35-editor",
            "delete",
            "{}",
            "--target",
            "segmentation[0]",
            "--list-targets",
        ]);
        let result = cli.run();
        assert!(result.is_ok());
    }

    #[test]
    fn output_format_conversion() {
        let format: OutputFormat = OutputFormatCli::Base64.into();
        assert!(matches!(format, OutputFormat::Base64));
        let format: OutputFormat = OutputFormatCli::Hex.into();
        assert!(matches!(format, OutputFormat::Hex));
        let format: OutputFormat = OutputFormatCli::Json.into();
        assert!(matches!(format, OutputFormat::Json));
    }

    #[test]
    fn input_format_conversion() {
        let format: InputFormat = InputFormatCli::Auto.into();
        assert!(matches!(format, InputFormat::Auto));
        let format: InputFormat = InputFormatCli::Json.into();
        assert!(matches!(format, InputFormat::Json));
        let format: InputFormat = InputFormatCli::Base64.into();
        assert!(matches!(format, InputFormat::Base64));
        let format: InputFormat = InputFormatCli::Hex.into();
        assert!(matches!(format, InputFormat::Hex));
    }
}

impl From<InputFormatCli> for InputFormat {
    fn from(value: InputFormatCli) -> Self {
        match value {
            InputFormatCli::Auto => InputFormat::Auto,
            InputFormatCli::Json => InputFormat::Json,
            InputFormatCli::Base64 => InputFormat::Base64,
            InputFormatCli::Hex => InputFormat::Hex,
        }
    }
}

impl From<OutputFormatCli> for OutputFormat {
    fn from(value: OutputFormatCli) -> Self {
        match value {
            OutputFormatCli::Json => OutputFormat::Json,
            OutputFormatCli::Base64 => OutputFormat::Base64,
            OutputFormatCli::Hex => OutputFormat::Hex,
        }
    }
}
