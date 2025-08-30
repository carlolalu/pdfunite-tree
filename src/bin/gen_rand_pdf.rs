use anyhow::{Result, anyhow};
use clap::Parser;
use pdfunite_tree::utils::get_basic_pdf_doc;
use std::path::Path;

/// Generate a PDF document with random content. The pages have for title the name of the document and the page number.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Output path
    output_path: String,
    /// Number of pages of the document
    num_pages: u8,
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = generate_basic_pdf_doc(cli) {
        println!("Error encountered: {}", err)
    }
}

fn generate_basic_pdf_doc(cli: Cli) -> Result<()> {
    let output_path = cli.output_path;
    let num_pages = cli.num_pages;

    if std::fs::exists(&output_path)? {
        return Err(anyhow!("A file at location '{output_path}' exists already"));
    }

    let doc_name = Path::new(&output_path)
        .file_name()
        .ok_or(anyhow!(
            "The output path provided does not present a filename"
        ))?
        .to_string_lossy()
        .to_string();
    let mut random_doc = get_basic_pdf_doc(&doc_name, num_pages)?;

    let mut buffer = Vec::new();
    random_doc.save_modern(&mut buffer)?;
    std::fs::write(&output_path, buffer)?;

    Ok(())
}
