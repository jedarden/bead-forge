use anyhow::Result;

fn main() -> Result<()> {
    let cli = bead_forge::cli::run_cli()?;
    bead_forge::cli::run(cli)
}
