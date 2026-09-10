use std::fs;
use std::io::{self, Write};
use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{CliError, CliTool};

use super::{decode_base64, encode_bytes, is_image_file_path, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "base64img",
        aliases: &["b64i"],
        about: "图片与 Base64 互转",
        configure: configure,
        run: run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("图片文件路径或 Base64 / data URI"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件；编码时省略则写到 stdout"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let output = matches.get_one::<String>("output").map(String::as_str);
    write_payload(output, convert_input(input)?)
}

enum OutputPayload {
    Text(String),
    Bytes(Vec<u8>),
}

fn convert_input(input: &str) -> Result<OutputPayload, CliError> {
    let path = Path::new(input);
    if path.is_file() {
        if is_image_file_path(path) {
            let bytes = fs::read(path).map_err(|_| CliError::new("无法读取输入文件"))?;
            Ok(OutputPayload::Text(encode_bytes(&bytes)))
        } else {
            let text = fs::read_to_string(path).map_err(|_| CliError::new("无法读取输入文件"))?;
            decode_payload(&text)
        }
    } else if path.exists() {
        Err(CliError::new("无法读取输入文件"))
    } else {
        decode_payload(input)
    }
}

fn decode_payload(text: &str) -> Result<OutputPayload, CliError> {
    let bytes = decode_base64(text).map_err(|err| CliError::new(err.to_string()))?;
    Ok(OutputPayload::Bytes(bytes))
}

fn write_payload_bytes(payload: OutputPayload, to_stdout: bool) -> Result<Vec<u8>, CliError> {
    Ok(match payload {
        OutputPayload::Text(text) => {
            let mut out = text.into_bytes();
            if to_stdout && !out.ends_with(b"\n") {
                out.push(b'\n');
            }
            out
        }
        OutputPayload::Bytes(bytes) => bytes,
    })
}

fn write_payload(output: Option<&str>, payload: OutputPayload) -> Result<(), CliError> {
    let bytes = write_payload_bytes(payload, output.is_none())?;
    match output {
        Some(path) => fs::write(path, bytes).map_err(|_| CliError::new("无法写入输出文件")),
        None => {
            let mut stdout = io::stdout().lock();
            stdout
                .write_all(&bytes)
                .map_err(|_| CliError::new("无法写入标准输出"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use clap::Command;
    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "devtoys-b64img-{}-{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn rgb_1x1_png() -> Vec<u8> {
        let img = RgbImage::from_pixel(1, 1, Rgb([10, 20, 30]));
        let mut buf = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        buf
    }

    fn invoke(args: &[&str]) -> Result<(), CliError> {
        let matches = (cli_tool().configure)(Command::new("base64img"))
            .try_get_matches_from(args)
            .expect("CLI args should parse");
        run(&matches)
    }

    fn assert_png_file(path: &Path, expected: &[u8]) {
        let got = fs::read(path).unwrap();
        assert_eq!(got, expected, "output file must be original image bytes");
        assert_eq!(
            image::guess_format(&got).unwrap(),
            ImageFormat::Png,
            "independent reader must see a PNG"
        );
        let loaded = image::load_from_memory(&got).unwrap().to_rgb8();
        assert_eq!(loaded.dimensions(), (1, 1));
        assert_eq!(loaded.get_pixel(0, 0), &Rgb([10, 20, 30]));
    }

    #[test]
    fn encodes_png_file_to_output_file() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let input = scratch.path().join("in.png");
        let output = scratch.path().join("out.txt");
        fs::write(&input, &png).unwrap();

        invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        let got = fs::read_to_string(&output).unwrap();
        assert_eq!(got, STANDARD.encode(&png));
        assert_eq!(STANDARD.decode(got.as_bytes()).unwrap(), png);
    }

    #[test]
    fn decodes_txt_file_to_png_output_file() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let encoded = STANDARD.encode(&png);
        let input = scratch.path().join("base64.txt");
        let output = scratch.path().join("out.png");
        fs::write(&input, &encoded).unwrap();

        invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        assert_png_file(&output, &png);
        let as_text = fs::read_to_string(&output);
        assert!(
            as_text.is_err() || !as_text.unwrap().starts_with(&encoded),
            "must decode, not re-encode the text file bytes"
        );
    }

    #[test]
    fn decodes_txt_file_with_surrounding_whitespace() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let input = scratch.path().join("padded.txt");
        let output = scratch.path().join("out.png");
        fs::write(&input, format!("  \r\n{}\n  ", STANDARD.encode(&png))).unwrap();

        invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        assert_png_file(&output, &png);
    }

    #[test]
    fn decodes_txt_file_with_data_uri() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let input = scratch.path().join("datauri.txt");
        let output = scratch.path().join("out.png");
        fs::write(
            &input,
            format!("data:image/png;base64,{}", STANDARD.encode(&png)),
        )
        .unwrap();

        invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        assert_png_file(&output, &png);
    }

    #[test]
    fn decodes_inline_base64_to_output_file() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let output = scratch.path().join("out.png");

        invoke(&[
            "base64img",
            "-i",
            &STANDARD.encode(&png),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        assert_png_file(&output, &png);
    }

    #[test]
    fn uppercase_png_extension_encodes() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let input = scratch.path().join("IN.PNG");
        let output = scratch.path().join("out.txt");
        fs::write(&input, &png).unwrap();

        invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        assert_eq!(fs::read_to_string(&output).unwrap(), STANDARD.encode(&png));
    }

    const MINIMAL_SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";

    #[test]
    fn encodes_svg_file_to_base64() {
        let scratch = Scratch::new();
        let input = scratch.path().join("in.svg");
        let output = scratch.path().join("out.txt");
        fs::write(&input, MINIMAL_SVG).unwrap();

        invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        let got = fs::read_to_string(&output).unwrap();
        assert_eq!(got, STANDARD.encode(MINIMAL_SVG));
        assert_eq!(STANDARD.decode(got.as_bytes()).unwrap(), MINIMAL_SVG);
    }

    #[test]
    fn decodes_svg_base64_to_svg_file() {
        let scratch = Scratch::new();
        let input = scratch.path().join("base64.txt");
        let output = scratch.path().join("out.svg");
        fs::write(&input, STANDARD.encode(MINIMAL_SVG)).unwrap();

        invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap();

        assert_eq!(fs::read(&output).unwrap(), MINIMAL_SVG);
        let text = fs::read_to_string(&output).unwrap();
        assert!(text.contains("<svg"), "{text}");
        assert!(text.contains("http://www.w3.org/2000/svg"), "{text}");
    }

    #[test]
    fn decodes_svg_data_uri_inline_to_file() {
        let scratch = Scratch::new();
        let output = scratch.path().join("out.svg");
        let uri = format!("data:image/svg+xml;base64,{}", STANDARD.encode(MINIMAL_SVG));

        invoke(&["base64img", "-i", &uri, "-o", output.to_str().unwrap()]).unwrap();

        assert_eq!(fs::read(&output).unwrap(), MINIMAL_SVG);
    }

    #[test]
    fn invalid_text_file_returns_error_and_does_not_write_output() {
        let scratch = Scratch::new();
        let input = scratch.path().join("bad.txt");
        let output = scratch.path().join("out.png");
        fs::write(&input, "not-an-image!!!").unwrap();

        let err = invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap_err();

        assert_eq!(err.message, "非法图片");
        assert!(!output.exists(), "failure must not write an output file");
    }

    #[test]
    fn unreadable_input_path_that_is_a_directory_fails() {
        let scratch = Scratch::new();
        let output = scratch.path().join("out.png");
        let err = invoke(&[
            "base64img",
            "-i",
            scratch.path().to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .unwrap_err();
        assert_eq!(err.message, "无法读取输入文件");
        assert!(!output.exists());
    }

    #[test]
    fn write_failure_to_directory_is_error() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let input = scratch.path().join("in.png");
        fs::write(&input, &png).unwrap();

        let err = invoke(&[
            "base64img",
            "-i",
            input.to_str().unwrap(),
            "-o",
            scratch.path().to_str().unwrap(),
        ])
        .unwrap_err();
        assert_eq!(err.message, "无法写入输出文件");
    }

    #[test]
    fn encode_image_file_stdout_payload_is_base64_text() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let input = scratch.path().join("in.png");
        fs::write(&input, &png).unwrap();

        match convert_input(input.to_str().unwrap()).unwrap() {
            OutputPayload::Text(text) => assert_eq!(text, STANDARD.encode(&png)),
            OutputPayload::Bytes(_) => panic!("image file must encode to text, not bytes"),
        }
    }

    #[test]
    fn decode_text_file_stdout_payload_is_png_bytes() {
        let scratch = Scratch::new();
        let png = rgb_1x1_png();
        let input = scratch.path().join("base64.txt");
        fs::write(&input, STANDARD.encode(&png)).unwrap();

        match convert_input(input.to_str().unwrap()).unwrap() {
            OutputPayload::Bytes(bytes) => {
                assert_eq!(bytes, png);
                assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Png);
            }
            OutputPayload::Text(_) => panic!("text file must decode to image bytes"),
        }
    }

    #[test]
    fn stdout_text_gets_trailing_newline_bytes_do_not() {
        let text = write_payload_bytes(OutputPayload::Text("abc".into()), true).unwrap();
        assert_eq!(text, b"abc\n");

        let png = rgb_1x1_png();
        let bytes = write_payload_bytes(OutputPayload::Bytes(png.clone()), true).unwrap();
        assert_eq!(bytes, png);
    }
}
