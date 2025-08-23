mod dev_playground;

use anyhow::Result;
use clap::Parser;

/// Merge together all the PDFs in a folder and its subfolders (max X levels) into a single document
/// provided with a ToC (Table fo Contents) reflecting the structure of tree of the folder and its descendants.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Folder containing the pdfs
    #[arg(short, long)]
    root_pdfs: Option<String>,
    #[command(subcommand)]
    dev_playground_command: Option<dev_playground::DevCommands>,
}


fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(playground_cmd) = cli.dev_playground_command {
        dev_playground::execute_cmd(&playground_cmd)?;
    }

    Ok(())
}
