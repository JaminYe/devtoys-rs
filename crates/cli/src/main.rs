use std::process::ExitCode;

use devtoys_tools::{
    build_cli_from, default_extensions_dir, load_extensions, run_cli_from, ToolCatalog,
};

fn main() -> ExitCode {
    let loaded = load_extensions(default_extensions_dir());
    for err in &loaded.errors {
        eprintln!("跳过扩展: {err}");
    }
    let catalog = ToolCatalog::with_extensions(loaded.tools);
    let mut cmd = build_cli_from(&catalog);
    let matches = cmd.get_matches_mut();
    if matches.subcommand().is_none() {
        let mut help = build_cli_from(&catalog);
        if help.print_help().is_err() {
            eprintln!("无法打印帮助");
            return ExitCode::from(1);
        }
        println!();
        return ExitCode::SUCCESS;
    }
    match run_cli_from(&catalog, &matches) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err.message);
            ExitCode::from(1)
        }
    }
}
