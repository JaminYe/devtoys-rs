use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{CliError, CliTool};

use super::{is_static_image_path, simulate_color_blindness, SimulatedImages, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "colorblindsimulator",
        aliases: &["cbs"],
        about: "模拟红色盲 / 绿色盲 / 黄蓝色盲并写出四张 PNG",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入图像路径"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出目录；省略则写到输入文件所在目录"),
    )
    .arg(
        Arg::new("silent")
            .short('s')
            .action(ArgAction::SetTrue)
            .help("不打印写出的文件路径"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let input_path = Path::new(input);
    if !input_path.is_file() {
        return Err(CliError::new("无法读取输入文件"));
    }
    if !input_path
        .to_str()
        .is_some_and(is_static_image_path)
    {
        return Err(CliError::new("不支持的图像类型"));
    }

    let bytes = fs::read(input_path).map_err(|_| CliError::new("无法读取输入文件"))?;
    let images = simulate_color_blindness(&bytes).map_err(|_| CliError::new("无法解码图像"))?;

    let out_dir = match matches.get_one::<String>("output") {
        Some(dir) => PathBuf::from(dir),
        None => input_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    fs::create_dir_all(&out_dir).map_err(|_| CliError::new("无法写入输出目录"))?;

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let written = write_four_pngs(&images, &out_dir, stem)?;
    if !matches.get_flag("silent") {
        for path in written {
            println!("{}", path.display());
        }
    }
    Ok(())
}

fn write_four_pngs(
    images: &SimulatedImages,
    out_dir: &Path,
    stem: &str,
) -> Result<Vec<PathBuf>, CliError> {
    let files = [
        (format!("{stem}-original.png"), &images.original),
        (format!("{stem}-protanopia.png"), &images.protanopia),
        (format!("{stem}-deuteranopia.png"), &images.deuteranopia),
        (format!("{stem}-tritanopia.png"), &images.tritanopia),
    ];
    let mut written = Vec::with_capacity(4);
    for (name, bytes) in files {
        let path = out_dir.join(name);
        fs::write(&path, bytes).map_err(|_| CliError::new("无法写入输出文件"))?;
        written.push(path);
    }
    Ok(written)
}
