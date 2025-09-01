use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use lopdf::Document;
use pdfunite_tree::utils::{get_basic_pdf_doc, get_catalog_children_names};
use std::path::Path;

/// Generate a PDF document with random content. The pages have for title the name of the document and the page number.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Desired action
    #[command(subcommand)]
    cmd: ToolCmd,
}

#[derive(Subcommand, Debug)]
enum ToolCmd {
    /// Generate random PDF with basic features
    GenerateRandomPdf {
        /// Output path
        #[arg(short = 'o')]
        output_path: String,
        /// Number of pages of the document
        #[arg(short = 'n')]
        num_pages: u8,
    },
    /// Show the names of all the children of the Catalog's PDF
    ShowCatalogChildren {
        /// Path of the pdf file
        pdf_path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.cmd {
        ToolCmd::GenerateRandomPdf {
            output_path,
            num_pages,
        } => generate_basic_pdf_doc(output_path, num_pages),
        ToolCmd::ShowCatalogChildren { pdf_path } => show_catalog_children_names(pdf_path),
    };

    if let Err(err) = result {
        println!("Error encountered: {}", err)
    }
}

fn generate_basic_pdf_doc(output_path: impl AsRef<Path>, num_pages: u8) -> Result<()> {
    let output_path = output_path.as_ref();

    if std::fs::exists(output_path)? {
        return Err(anyhow!(
            "A file at location '{}' exists already",
            output_path.display()
        ));
    }

    let doc_name = output_path
        .file_name()
        .ok_or(anyhow!(
            "The output path provided does not present a filename"
        ))?
        .to_string_lossy()
        .to_string();
    let mut random_doc = get_basic_pdf_doc(&doc_name, num_pages)?;

    let mut buffer = Vec::new();
    random_doc.save_modern(&mut buffer)?;
    std::fs::write(output_path, buffer)?;

    Ok(())
}

fn show_catalog_children_names(input_path: impl AsRef<Path>) -> Result<()> {
    let input_path = input_path.as_ref();

    let catalog_children_names = get_catalog_children_names(&Document::load(input_path)?)?;

    println!("Catalog");
    let num_children = catalog_children_names.len();

    for (index, child) in catalog_children_names.iter().enumerate() {
        let prefix = if index < num_children - 1 {
            "├── ".to_string()
        } else {
            "└── ".to_string()
        };
        println!("{prefix}{child}");
    }

    Ok(())
}
