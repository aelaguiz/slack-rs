use std::process::ExitCode;

fn main() -> ExitCode {
    // Local development convenience. If `.env` is missing, that's fine — env can come from the shell.
    dotenvy::dotenv().ok();

    match parse_mode_and_run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Best-effort terminal restore so the error is readable.
            slack_rs::terminal::restore_best_effort();
            slack_rs::diagnostics::banner::print_fatal_error(&err);
            ExitCode::from(1)
        }
    }
}

fn parse_mode_and_run() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);

    let mut headless = false;
    let mut script_path: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--script" => {
                script_path = Some(args.next().ok_or_else(|| {
                    anyhow::anyhow!("missing value for `--script` (use `-` for stdin)")
                })?);
            }
            _ => {
                anyhow::bail!(
                    "unknown arg: {arg}\n\
                    Supported:\n\
                    - (interactive) no args\n\
                    - (headless) `--headless --script <path|->`"
                );
            }
        }
    }

    if headless {
        let script_path = script_path.unwrap_or_else(|| "-".to_string());
        return slack_rs::app::headless::run_headless(slack_rs::app::headless::HeadlessArgs {
            script_path,
        });
    }

    slack_rs::app::app::App::run()
}
