use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum InputSpec {
    Inline(String),
    File(PathBuf),
    Stdin,
}

#[derive(Copy, Clone, Debug)]
pub enum InputFormat {
    Auto,
    Json,
    Base64,
    Hex,
}

#[derive(Clone, Debug)]
pub enum OutputTarget {
    Stdout,
    File(PathBuf),
}

impl OutputTarget {
    pub fn from_path(path: Option<&Path>) -> Self {
        match path {
            Some(path) => OutputTarget::File(path.to_path_buf()),
            None => OutputTarget::Stdout,
        }
    }
}

pub fn load_input(spec: &InputSpec) -> Result<String, String> {
    match spec {
        InputSpec::Inline(value) => Ok(value.clone()),
        InputSpec::File(path) => fs::read_to_string(path)
            .map_err(|err| format!("failed to read input file {}: {err}", path.display())),
        InputSpec::Stdin => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|err| format!("failed to read stdin: {err}"))?;
            Ok(buffer)
        }
    }
}

pub fn write_output(target: OutputTarget, output: &str) -> Result<(), String> {
    match target {
        OutputTarget::Stdout => {
            println!("{output}");
            Ok(())
        }
        OutputTarget::File(path) => fs::write(&path, output)
            .map_err(|err| format!("failed to write output file {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_inline_input() {
        let spec = InputSpec::Inline("test".to_string());
        let result = load_input(&spec).expect("load failed");
        assert_eq!(result, "test");
    }

    #[test]
    fn read_file_input() {
        let path = PathBuf::from("target/tmp-input.txt");
        fs::create_dir_all("target").expect("mkdir failed");
        fs::write(&path, "file").expect("write failed");
        let spec = InputSpec::File(path.clone());
        let result = load_input(&spec).expect("load failed");
        assert_eq!(result, "file");
    }

    #[test]
    fn write_stdout() {
        let result = write_output(OutputTarget::Stdout, "hello");
        assert!(result.is_ok());
    }

    #[test]
    fn write_file_output() {
        let path = PathBuf::from("target/tmp-output.txt");
        let result = write_output(OutputTarget::File(path.clone()), "out");
        assert!(result.is_ok());
        let contents = fs::read_to_string(&path).expect("read failed");
        assert_eq!(contents, "out");
    }

    #[test]
    fn read_file_input_missing() {
        let spec = InputSpec::File(PathBuf::from("target/does-not-exist.txt"));
        let err = load_input(&spec).expect_err("expected error");
        assert!(err.contains("failed to read input file"));
    }

    #[test]
    fn read_stdin_input() {
        let spec = InputSpec::Stdin;
        let result = load_input(&spec).expect("load failed");
        assert!(result.is_empty());
    }

    #[test]
    fn write_file_output_missing_dir() {
        let path = PathBuf::from("target/no-such-dir/out.txt");
        let err = write_output(OutputTarget::File(path), "out").expect_err("expected error");
        assert!(err.contains("failed to write output file"));
    }
}
