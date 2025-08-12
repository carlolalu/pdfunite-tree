use anyhow::Result;
use clap::{Parser, Subcommand};

/// Merge together all the PDFs in a folder and its subfolders (max X levels) into a single document
/// provided with a ToC (Table fo Contents) reflecting the structure of tree of the folder and its descendants.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Folder containing the pdfs
    #[arg(short, long)]
    root_pdfs: Option<String>,
    #[command(subcommand)]
    dev_command: Option<DevCommands>,
}

#[derive(Subcommand, Debug)]
enum DevCommands {
    /// Generate a basic PDF
    GenerateBasicPDF {
        /// Document name
        #[arg(short, long)]
        name: String,
        /// Number of pages
        #[arg(short, long, default_value_t = 2)]
        pages: u8,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(dev_command) = cli.dev_command {
        match dev_command {
            DevCommands::GenerateBasicPDF { name, pages } => {
                dev_exercises::generate_basic_pdf(&name, pages)?;
            }
        }
    }

    Ok(())
}

mod dev_exercises {
    use anyhow::Result;
    use lopdf::{
        self, Document, Object, ObjectId, Stream,
        content::{Content, Operation},
        dictionary,
    };

    /// Generate a PDF file with minimal features
    pub fn generate_basic_pdf(doc_name: &str, pages: u8) -> Result<()> {
        let mut doc = Document::with_version("1.5");

        let pages_id = doc.new_object_id();

        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Courier",
        });

        let resources_id = doc.add_object(dictionary! {
                "Font" => dictionary! {
                    "F1" => font_id,
                },

        });

        let pages_ids: Vec<_> = (1..=pages)
            .map(|page_number| {
                append_random_page_to_doc(page_number, doc_name, &pages_id, &mut doc)
            })
            .collect::<Result<_>>()?;

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => pages_ids.iter().map(|&page_id| page_id.into()).collect::<Vec<_>>(),
            "Count" => pages,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };

        // Using `insert()` here, instead of `add_object()` since the ID is already known.
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });

        // The "Root" key in trailer is set to the ID of the document catalog,
        // the remainder of the trailer is set during `doc.save()`.
        doc.trailer.set("Root", catalog_id);
        doc.compress();

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let parent_folder = format!("{manifest_dir}/{DEV_DATA_FOLDER}/generate_basic_pdf");
        std::fs::create_dir_all(&parent_folder)?;

        let doc_name = if doc_name.ends_with(".pdf") {
            let doc_name = doc_name.replace(".pdf", "");
            format!("{doc_name}-with-toc.pdf")
        } else {
            format!("{doc_name}-with-toc.pdf")
        };

        let filepath = format!("{parent_folder}/{doc_name}");
        doc.save(filepath)?;

        Ok(())
    }

    fn append_random_page_to_doc(
        page_number: u8,
        doc_name: &str,
        pages_id: &ObjectId,
        doc: &mut Document,
    ) -> Result<ObjectId> {
        let page_title = format!("Page {page_number}");
        let random_text = craft_random_text_of_len(20);

        // Wrapper for a vector of operands and operations in PDF
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Td", vec![50.into(), 600.into()]),
                Operation::new("TL", vec![50.into()]),
                Operation::new("Tf", vec!["F1".into(), 46.into()]),
                Operation::new("Tj", vec![Object::string_literal(doc_name)]),
                Operation::new("Tf", vec!["F1".into(), 36.into()]),
                Operation::new("'", vec![Object::string_literal(page_title)]),
                Operation::new("Tf", vec!["F1".into(), 20.into()]),
                Operation::new("'", vec![Object::string_literal(random_text)]),
                Operation::new("ET", vec![]),
            ],
        };

        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => *pages_id,
            "Contents" => content_id,
        });

        Ok(page_id)
    }

    fn craft_random_text_of_len(char_length: usize) -> String {
        use rand::distr::{SampleString, StandardUniform};

        let random_string: String = StandardUniform.sample_string(&mut rand::rng(), char_length);
        //println!("random_valid_text: [[[{random_valid_text}]]]");

        random_string
    }
}
