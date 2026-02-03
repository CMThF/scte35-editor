mod app;
mod ui;

use crate::core::{ParseSettings, Scte35Document};
use crate::io::InputSpec;

pub use app::InteractiveArgs;

pub fn run(args: &InteractiveArgs) -> Result<(), String> {
    let (document, start_requires_template) = if let Some(input) = args.input.as_deref() {
        let input_spec = InputSpec::Inline(input.to_string());
        let input = crate::io::load_input(&input_spec)?;
        let document = Scte35Document::parse(
            &input,
            args.input_format,
            ParseSettings {
                validate_crc: !args.no_crc,
            },
        )?;
        (document, false)
    } else if let Some(path) = args.file.as_deref() {
        let input_spec = InputSpec::File(path.to_path_buf());
        let input = crate::io::load_input(&input_spec)?;
        let document = Scte35Document::parse(
            &input,
            args.input_format,
            ParseSettings {
                validate_crc: !args.no_crc,
            },
        )?;
        (document, false)
    } else {
        let document = Scte35Document::new_default_time_signal()?;
        (document, true)
    };

    let output_target = args.output_file.clone();
    app::run_app(
        document,
        args.output_format,
        output_target,
        start_requires_template,
    )
}
