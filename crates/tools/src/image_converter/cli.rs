use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};

use super::is_static_image_path;
use crate::cli::{CliError, CliTool};

use super::execute::convert_paths;
use super::{static_images_in_dir, ImageTargetFormat, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "imageconverter",
        aliases: &["imgconv"],
        about: "在 BMP / JPEG / PBM / PNG / TGA / TIFF / WEBP 间转换",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入文件或目录（目录只读直接子文件，不递归）"),
    )
    .arg(
        Arg::new("format")
            .short('t')
            .required(true)
            .value_parser(["Bmp", "Jpeg", "Pbm", "Png", "Tga", "Tiff", "Webp"])
            .help("目标格式"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件或目录；省略则写到输入旁"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let format = matches
        .get_one::<String>("format")
        .ok_or_else(|| CliError::new("缺少目标格式"))?;
    let format = ImageTargetFormat::parse(format).ok_or_else(|| CliError::new("未知格式"))?;
    let output = matches.get_one::<String>("output").map(PathBuf::from);
    let input_path = Path::new(input);

    if input_path.is_dir() {
        convert_directory(input_path, format, output.as_deref())
    } else if input_path.is_file() {
        convert_one_file(input_path, format, output.as_deref())
    } else {
        Err(CliError::new("无法读取输入"))
    }
}

fn convert_one_file(
    input: &Path,
    format: ImageTargetFormat,
    output: Option<&Path>,
) -> Result<(), CliError> {
    if !input.to_str().is_some_and(is_static_image_path) {
        return Err(CliError::new("不支持的图像类型"));
    }
    let path = input.to_string_lossy().to_string();
    let batch = convert_paths(&[path], format);
    let converted = match batch.successes.into_iter().next() {
        Some(success) => success.bytes,
        None => {
            let msg = batch
                .failures
                .first()
                .map(|f| f.error.clone())
                .unwrap_or_else(|| "无法转换图像".into());
            return Err(CliError::new(msg));
        }
    };
    let dest = match output {
        Some(path) if path.is_dir() || path_is_dir_hint(path) => {
            fs::create_dir_all(path).map_err(|_| CliError::new("无法写入输出目录"))?;
            path.join(output_name(input, format))
        }
        Some(path) => {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent).map_err(|_| CliError::new("无法写入输出文件"))?;
            }
            path.to_path_buf()
        }
        None => input.with_extension(format.extension()),
    };
    fs::write(&dest, converted).map_err(|_| CliError::new("无法写入输出文件"))
}

fn convert_directory(
    input: &Path,
    format: ImageTargetFormat,
    output: Option<&Path>,
) -> Result<(), CliError> {
    let dest_dir = match output {
        Some(path) => {
            fs::create_dir_all(path).map_err(|_| CliError::new("无法写入输出目录"))?;
            path.to_path_buf()
        }
        None => input.to_path_buf(),
    };
    let files = static_images_in_dir(input).map_err(|_| CliError::new("无法读取输入目录"))?;
    if files.is_empty() {
        return Err(CliError::new("目录中没有静态图像"));
    }
    for file in &files {
        let path = file.to_string_lossy().to_string();
        let batch = convert_paths(&[path], format);
        let converted = match batch.successes.into_iter().next() {
            Some(success) => success.bytes,
            None => {
                let msg = batch
                    .failures
                    .first()
                    .map(|f| f.error.clone())
                    .unwrap_or_else(|| "无法转换图像".into());
                return Err(CliError::new(msg));
            }
        };
        let dest = dest_dir.join(output_name(file, format));
        fs::write(&dest, converted).map_err(|_| CliError::new("无法写入输出文件"))?;
    }
    Ok(())
}

fn output_name(input: &Path, format: ImageTargetFormat) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    PathBuf::from(format!("{stem}.{}", format.extension()))
}

fn path_is_dir_hint(path: &Path) -> bool {
    path.as_os_str().to_string_lossy().ends_with(['/', '\\'])
}
