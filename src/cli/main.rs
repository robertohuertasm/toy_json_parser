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
    /// If set, the file will be read by chunks. It works best for heavy files. If your file is not that big don't set this property as it will usually work faster.
    #[structopt(short = "c", long)]
    pub use_chunks: bool,
    /// It defines the chunk size that the tool will use to read the file in chunks.
    #[structopt(long, default_value = "1000000")]
    pub chunk_size: usize,
    /// If set, the result will be displayed in a pretty table
    #[structopt(short = "p", long)]
    pub pretty_print: bool,
    /// If set, some additional errors will be derived to the stderr
    #[structopt(short = "v", long)]
    pub verbose_errors: bool,
}

fn main() -> std::io::Result<()> {
    let cli: Cli = Cli::from_args();
    let current_dir = std::env::current_dir()?;
    let path = current_dir.join(cli.file_path);
    file_reader::start(
        path,
        cli.pretty_print,
        cli.use_chunks,
        cli.chunk_size,
        cli.verbose_errors,
    );
    Ok(())
}
