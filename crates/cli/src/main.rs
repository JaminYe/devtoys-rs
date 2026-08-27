use std::process::ExitCode;

use devtoys_tools::{build_cli, run_cli};

fn main() -> ExitCode {
    let mut cmd = build_cli();
    let matches = cmd.get_matches_mut();
    if matches.subcommand().is_none() {
        let mut help = build_cli();
        if help.print_help().is_err() {
            eprintln!("无法打印帮助");
            return ExitCode::from(1);
        }
        println!();
        return ExitCode::SUCCESS;
    }
    match run_cli(&matches) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err.message);
            ExitCode::from(1)
        }
    }
}
