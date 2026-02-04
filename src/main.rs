#[cfg(feature = "tui")]
use clap::Parser;
#[cfg(feature = "tui")]
use scte35_editor::cli::Cli;

fn main() {
    #[cfg(feature = "tui")]
    {
        if let Err(err) = run_with_args(std::env::args()) {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
    #[cfg(not(feature = "tui"))]
    {
        eprintln!("scte35-editor built without TUI/CLI support");
        std::process::exit(1);
    }
}

#[cfg(feature = "tui")]
fn run_with_args<I, T>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    cli.run()
}

#[cfg(test)]
#[cfg(feature = "tui")]
mod tests {
    use super::run_with_args;

    #[test]
    fn run_with_args_help() {
        let args = ["scte35-editor", "list-paths"];
        let result = run_with_args(args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_with_args_error() {
        let args = ["scte35-editor", "edit"];
        let err = run_with_args(args).expect_err("expected error");
        assert!(err.contains("expected INPUT"));
    }
}
