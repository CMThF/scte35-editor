use clap::Parser;
use scte35_editor::cli::Cli;

fn main() {
    if let Err(err) = run_with_args(std::env::args()) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run_with_args<I, T>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    cli.run()
}

#[cfg(test)]
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
