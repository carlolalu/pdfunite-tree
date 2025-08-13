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
    /// Equip an already existing PDF file with a trivial ToC (each page is bookmarked in the Outline)
    EquipWithTrivialToc {
        /// Document path
        #[arg(short, long)]
        doc_path: String,
    },
    Experiment {
        /// Document path
        #[arg(short, long)]
        doc_path: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(dev_command) = cli.dev_command {
        match dev_command {
            DevCommands::GenerateBasicPDF { name, pages } => {
                dev_exercises::generate_basic_pdf(&name, pages)?;
            }
            DevCommands::EquipWithTrivialToc { doc_path } => {
                dev_exercises::equip_with_trivial_toc(&doc_path)?;
            }
            DevCommands::Experiment { doc_path } => {
                dev_exercises::experiment_to_equip_trivial_doc(&doc_path)?;
            }
        }
    }

    Ok(())
}

mod dev_exercises {
    use anyhow::{Result, anyhow};
    use lopdf::{
        self, Bookmark, Dictionary, Document, Object, ObjectId, Outline, Stream,
        content::{Content, Operation},
        dictionary,
    };
    use std::{collections::BTreeMap, path::Path};

    const DEV_PLAYGROUND_DIR: &str = "dev-playground";

    pub fn experiment_to_equip_trivial_doc(doc_path: &str) -> Result<()> {
        let mut doc = Document::load(doc_path)?;

        match doc.get_toc() {
            Ok(_toc) => return Err(anyhow!("A ToC is already present on such document")),
            Err(_no_outline) => (),
        }

        let outlines_id = doc.new_object_id();

        let outline_items_ids: Vec<_> = doc
            .get_pages()
            .iter()
            .map(|(page_numer, page_id)| {
                doc.add_bookmark(
                    Bookmark::new("Page".to_string(), [0f32; 3], 0, *page_id),
                    None,
                )
            })
            .collect();

        let outlines_id = doc
            .build_outline()
            .ok_or(anyhow!("The doc bookmarks table could not be built"))?;
        let catalog = doc.catalog_mut()?;

        catalog.set("Outlines", Object::Reference(outlines_id));
        catalog.set(
            "PageMode",
            Object::String("UseOutlines".into(), lopdf::StringFormat::Literal),
        );

        let doc_name = Path::new(doc_path)
            .file_name()
            .ok_or(anyhow!("The path provided does not point to a file"))?
            .to_str()
            .ok_or(anyhow!(
                "The path provided contains non-UTF8 chars (not supported)"
            ))?
            .to_string();

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let parent_folder = format!("{manifest_dir}/{DEV_PLAYGROUND_DIR}/equip_with_trivial_toc");
        std::fs::create_dir_all(&parent_folder)?;

        let doc_name = format!("{}-with-outline.pdf", doc_name);
        let filepath = format!("{parent_folder}/{doc_name}");
        doc.save(filepath)?;

        Ok(())
    }

    pub fn equip_with_trivial_toc(doc_path: &str) -> Result<()> {
        let mut doc = Document::load(doc_path)?;

        match doc.get_toc() {
            Ok(_toc) => return Err(anyhow!("A ToC is already present on such document")),
            Err(_no_outline) => (),
        }

        // todo: add verification for PDF metadata on the document, and return error if they
        // are present, because they will not be copied to the new document

        // objects may have an id: (object_number, object_generation)

        let outlines_id = doc.new_object_id();

        // Outlines is directly a child of the catalog (=root) with such 4 entries:
        // type Outlines
        // First and Last: the references to the linked list of the first and last outline items (thus referencing the object ids of such)
        // count the total number of entries in the whole outlines tree

        // outlines contains outline-items
        // at each level of the hierarchy the outline-items are a linked list. One can access teh beginning or end of hte linked list
        // from his parent with the keyword first and last

        // the outline items are structured so:
        // title String
        // parent: indirect reference to the parent
        // Prev - Next : the indirect references to the previous and next level in the list of same level outline items
        // First - Last : as for outlines, the references of the linked list below, if present. Otherwise not required
        // Count: count all the descendants, if any. Otherwise not required
        // Dest: name, byte string or array to specify a destination

        // what is a destination?
        // what I need in my case is this syntax:
        // [ page /Fit], where page is a reference to a page, ie the page id
        // or eventually this one
        // [ page /XYZ left top zoom], with left top zoom coords which can be set to 0/null

        // it is not clear if I should also use the option PageMode set to UseOutlines as a value in the catalog node

        let pages_ids: Vec<_> = doc
            .get_pages()
            .iter()
            .map(|(_page_number, page_id)| page_id)
            .cloned()
            .collect();

        let proto_outlines: Vec<_> = pages_ids
            .iter()
            .map(|page_id| (page_id, doc.new_object_id()))
            .collect();

        let first_outline_id = proto_outlines
            .first()
            .ok_or(anyhow!("The proto oulines vector has no memebers!"))?
            .1;
        let last_outline_id = proto_outlines
            .last()
            .ok_or(anyhow!("The proto oulines vector has no memebers!"))?
            .1;

        for (index, proto_outline) in proto_outlines.iter().enumerate() {
            let (page_id, outline_item_id) = proto_outline;

            let prev = if index == 0 {
                None
            } else {
                proto_outlines
                    .get(index - 1)
                    .map(|(_page_id, outline_id)| outline_id)
            };

            let next = if index == proto_outlines.len() {
                None
            } else {
                proto_outlines
                    .get(index + 1)
                    .map(|(_page_id, outline_id)| outline_id)
            };

            let outline_item = dictionary! {
                "Title" => Object::String(format!("Page {}", index + 1).as_bytes().to_vec(), lopdf::StringFormat::Literal),
                "Parent" => Object::Reference(outlines_id),
                "Prev" => match prev {
                    None => Object::Null,
                    Some(prev) => Object::Reference(*prev),
                },
                "Next" => match next {
                    None => Object::Null,
                    Some(next) => Object::Reference(*next),
                },
                "Dest" => Object::Array(vec![Object::Reference(**page_id), "XYZ".into(), Object::Null,Object::Null,Object::Null,]),

            };

            doc.set_object(*outline_item_id, outline_item);
        }

        let outlines = dictionary! {
            "Type" => Object::Name("Outlines".as_bytes().to_vec()),
            "First" => Object::Reference(first_outline_id),
            "Last" => Object::Reference(last_outline_id),
            "Count" => Object::Integer(proto_outlines.len() as i64),
        };

        doc.set_object(outlines_id, outlines);

        let catalog = doc.catalog_mut()?;

        catalog.set("Outlines", Object::Reference(outlines_id));
        catalog.set(
            "PageMode",
            Object::String("UseOutlines".into(), lopdf::StringFormat::Literal),
        );

        let doc_name = Path::new(doc_path)
            .file_name()
            .ok_or(anyhow!("The path provided does not point to a file"))?
            .to_str()
            .ok_or(anyhow!(
                "The path provided contains non-UTF8 chars (not supported)"
            ))?
            .to_string();

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let parent_folder = format!("{manifest_dir}/{DEV_PLAYGROUND_DIR}/equip_with_trivial_toc");
        std::fs::create_dir_all(&parent_folder)?;

        let doc_name = format!("{}-with-outline.pdf", doc_name);
        let filepath = format!("{parent_folder}/{doc_name}");
        doc.save(filepath)?;

        Ok(())
    }

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
        let parent_folder = format!("{manifest_dir}/{DEV_PLAYGROUND_DIR}/generate_basic_pdf");
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

    fn append_random_page_to_doc(page_number: u8, doc_name: &str, pages_id: &ObjectId, doc: &mut Document) -> Result<ObjectId> {
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
