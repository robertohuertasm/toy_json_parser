use structopt::StructOpt;

#[derive(StructOpt, PartialEq, Debug)]
#[structopt(
    name("🌇  File Reader"),
    author("💻  Roberto Huertas <roberto.huertas@outlook.com>"),
    long_about("🧰  Utility to parse JSON lines from a file")
)]
pub struct Cli {
    /// Path to your file
    #[structopt()]
    pub file_path: String,
    /// If set, the result will be displayed in a pretty table
    #[structopt(short = "p", long)]
    pub pretty_print: bool,
}

fn main() -> std::io::Result<()> {
    let cli: Cli = Cli::from_args();
    let current_dir = std::env::current_dir()?;
    let path = current_dir.join(cli.file_path);
    file_reader::start(path, cli.pretty_print);
    Ok(())
}
