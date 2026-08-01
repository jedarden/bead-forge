use anyhow::Result;

fn main() -> Result<()> {
    // --version / -V is a normal CLI flag handled by `cli::run`, which prints
    // "bf <version>" to stdout and returns Ok (exit 0), so there is no
    // pre-parse hook here.
    let cli = bead_forge::cli::run_cli()?;
    bead_forge::cli::run(cli)
}
