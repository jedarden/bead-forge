use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    eprintln!("Args: {:?}", args);
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("bf 0.2.0");
        eprintln!("Manual version handler triggered");
        std::process::exit(0);
    }
    eprintln!("Would call clap parsing here");
}
