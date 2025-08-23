use anyhow::Result;
use clap::Parser;

/// Merge together all the PDFs in a folder and its subfolders (max X levels) into a single document
/// provided with a ToC (Table fo Contents) reflecting the structure of tree of the folder and its descendants.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Directory containing the pdfs. If a development playground subcommand is called
    /// such path becomes the target on which the subcommand is called
    #[arg(short, long)]
    target_path: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    Ok(())
}
