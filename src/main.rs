use anyhow::Result;

fn main() -> Result<()> {
    // Handle version flag before clap parsing to output to stdout
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("bf {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let cli = bead_forge::cli::run_cli()?;
    bead_forge::cli::run(cli)
}
