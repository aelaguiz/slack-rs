pub fn print_fatal_error(err: &anyhow::Error) {
    eprintln!();
    eprintln!("slack-rs: FATAL ERROR (fail-fast)");
    eprintln!();
    eprintln!("{err:#}");
    eprintln!();
    eprintln!("Hint: rerun with `RUST_BACKTRACE=1` for a backtrace.");
}
