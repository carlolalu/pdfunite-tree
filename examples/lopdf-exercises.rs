use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use lopdf::{self, Bookmark, Document, Object, dictionary};
use std::path::Path;

const DEV_PLAYGROUND_DIR: &str = "dev-playground";

/// Merge together all the PDFs in a folder and its subfolders (max X levels) into a single document
/// provided with a ToC (Table fo Contents) reflecting the structure of tree of the folder and its descendants.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Directory containing the pdfs. If a development playground subcommand is called
    /// such path becomes the target on which the subcommand is called
    #[arg(short, long)]
    target_path: String,
    #[command(subcommand)]
    dev_playground_command: DevCommand,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    execute_cmd(&cli.dev_playground_command, &cli.target_path)?;

    Ok(())
}

#[derive(Subcommand, Debug)]
pub enum DevCommand {
    /// Equip an already existing PDF file with a trivial ToC (each page is bookmarked in the Outline)
    EquipWithTrivialToc,
    /// Equip an already existing PDF file with a trivial ToC (each page is bookmarked in the Outline)
    /// but with high level functions
    ExperimentEquipTrivialToc,
}

pub fn execute_cmd(playground_cmd: &DevCommand, target_path: &str) -> Result<()> {
    // todo: add verification that the target path is in the playground
    // todo: convert every 'string' to a 'impl AsRef<Path>' and act consequently

    match playground_cmd {
        DevCommand::EquipWithTrivialToc => equip_with_trivial_toc(target_path),
        DevCommand::ExperimentEquipTrivialToc => experiment_to_equip_trivial_doc(target_path),
    }
}

fn experiment_to_equip_trivial_doc(doc_path: &str) -> Result<()> {
    let mut doc = Document::load(doc_path)?;

    match doc.get_toc() {
        Ok(_toc) => return Err(anyhow!("A ToC is already present on such document")),
        Err(_no_outline) => (),
    }

    let _ = doc.get_pages().values().map(|page_id| {
        doc.add_bookmark(
            Bookmark::new("Page".to_string(), [0f32; 3], 0, *page_id),
            None,
        )
    });

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

fn equip_with_trivial_toc(doc_path: &str) -> Result<()> {
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

    let pages_ids: Vec<_> = doc.get_pages().values().cloned().collect();

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
