pub mod utils;

use anyhow::{Result, anyhow};
use clap::Parser;
use lazy_static::lazy_static;
use log::{info, trace};
use lopdf::{Bookmark, Document, Object, dictionary};
use std::path::Path;

const MAX_DEPTH_PDF_TREE: u8 = 5;
const DEFAULT_OUTPUT_SUFFIX: &str = "-united.pdf";

const DEFAULT_TEXT_FORMAT: u32 = 0;
const UNINITIALISED_PAGE_ID: (u32, u16) = (0, 0);
const BLACK_COLOR_RGB: [f32; 3] = [0f32; 3];

lazy_static! {
    static ref ALLOWED_CATALOG_CHILDREN_FOR_INPUT_PDF: Vec<String> =
        ["Type", "Version", "Pages", "PageMode"]
            .map(|not_owned| not_owned.to_string())
            .into_iter()
            .collect();
}

/// Merge together all the PDFs in the input directory and its subdirectories (max 5 levels) into a single document.
/// If the flag `with-outlines`` is activated, the output file will be provided with a ToC (Table of Contents)
/// reflecting the structure of tree of the directory and its descendants. The tool does NOT modify the input
/// directory and its content.
///
/// Assumptions on the pdf tree:
/// 1. The tree has not more than 5 levels (the root is considered level 0).
/// 2. All the files in the input directory and its subdirectories are PDFs and their names are UT8-encoded.
/// 3. The PDFs in the directory and its subdirectories have at most these features:
///     * Pages
///     * PageMode
///
// (todo: specify rather which features are supported, and add more to them, otherwise is kind of lame).
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Directory containing the pdfs
    input_directory: String,
    /// Output path (must not be among the descendants of the input-directory)
    #[arg(short = 'o')]
    output_path: Option<String>,
    /// Provide the output file with a ToC (Oulines/Bookmark)
    /// reflecting the tree structure of the input directory.
    #[arg(short, long, default_value_t = true)]
    with_outlines: bool,
}

pub fn run() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let mut target_dir_path = cli.input_directory;
    let target_dir_path = if target_dir_path.ends_with('/') {
        target_dir_path.pop();
        Path::new(&target_dir_path)
    } else {
        Path::new(&target_dir_path)
    }
    .canonicalize()?;

    let output_path = cli.output_path.unwrap_or(format!(
        "{}{DEFAULT_OUTPUT_SUFFIX}",
        target_dir_path.display()
    ));
    let output_path = Path::new(&output_path);

    if output_path.starts_with(&target_dir_path) {
        return Err(anyhow!(
            "The output file cannot be a descendant of the input directory: \
            '{}' is a descendant of '{}'",
            output_path.display(),
            target_dir_path.display()
        ));
    }

    let mut main_doc = get_merged_tree_doc(target_dir_path, cli.with_outlines)?;

    main_doc.compress();

    if std::fs::exists(output_path)? {
        return Err(anyhow!(
            "A file '{}' is already present",
            output_path.display()
        ));
    } else {
        main_doc.save(output_path)?;
        println!("Output document saved as '{}'", output_path.display());
    }

    Ok(())
}

fn get_merged_tree_doc(target_dir_path: impl AsRef<Path>, with_outlines: bool) -> Result<Document> {
    let target_dir_path = target_dir_path.as_ref();

    info!("Initialising main document");
    let mut main_doc = Document::with_version("1.7");
    initialise_doc_with_null_pages(&mut main_doc)?;

    info!("Start the merging process");
    merge_from_internal_node(&mut main_doc, target_dir_path, 0, None)?;

    if with_outlines {
        main_doc.adjust_zero_pages();
        info!("Build the Outline of the main document and append it to the catalog");
        let outlines_id = main_doc.build_outline().ok_or(anyhow!(
            "The Outlines object for the document obtained is empty"
        ))?;
        let catalog = main_doc.catalog_mut()?;
        catalog.set("Outlines", Object::Reference(outlines_id));
        catalog.set(
            "PageMode",
            Object::String("UseOutlines".into(), lopdf::StringFormat::Literal),
        );
    }

    Ok(main_doc)
}

fn initialise_doc_with_null_pages(doc: &mut Document) -> Result<()> {
    let main_pages_root = dictionary!(
        b"Type" => Object::Name(b"Pages".to_vec()),
        b"Kids" => Object::Array(vec![]),
        b"Count" => Object::Integer(0)
    );

    let main_root_pages_id = doc.add_object(Object::Dictionary(main_pages_root));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(main_root_pages_id)
    });
    doc.trailer.set("Root", catalog_id);

    Ok(())
}

fn merge_from_internal_node(
    main_doc: &mut Document,
    directory: impl AsRef<Path>,
    parent_level: u8,
    parent_bookmark_id: Option<u32>,
) -> Result<()> {
    trace!(
        "Merge the node (=symlink or directory) '{}' and add its bookmark",
        directory.as_ref().display()
    );

    if parent_level > MAX_DEPTH_PDF_TREE {
        return Err(anyhow!(
            "The number of levels achieved is higher than the maximum \
            allowed (={MAX_DEPTH_PDF_TREE}): {parent_level}"
        ));
    }

    let mut entries = std::fs::read_dir(directory.as_ref())?
        .map(|res| match res {
            Ok(dir_entry) => Ok(dir_entry),
            Err(err) => Err(anyhow!("{err}")),
        })
        .collect::<Result<Vec<_>>>()?;

    if entries.is_empty() {
        trace!(
            "The node (=symlink or directory) '{}' is empty, therefore its bookmark is not added",
            directory.as_ref().display()
        );
        return Ok(());
    }

    let node_bookmark_id = {
        let dir_name = directory
            .as_ref()
            .file_name()
            .ok_or(anyhow!(
                "Could not get name of the directory '{}'",
                directory.as_ref().display()
            ))?
            .to_string_lossy()
            .to_string();

        let node_bookmark = Bookmark::new(
            dir_name,
            BLACK_COLOR_RGB,
            DEFAULT_TEXT_FORMAT,
            UNINITIALISED_PAGE_ID,
        );
        Some(main_doc.add_bookmark(node_bookmark, parent_bookmark_id))
    };

    entries.sort_by_key(|dir_entry| dir_entry.path());
    for entry in entries {
        let file_type = entry.file_type()?;

        if file_type.is_file() {
            merge_from_leaf(main_doc, entry.path(), node_bookmark_id)?;
        } else {
            merge_from_internal_node(main_doc, entry.path(), parent_level + 1, node_bookmark_id)?;
        }
    }

    Ok(())
}

fn merge_from_leaf(
    main_doc: &mut Document,
    path_doc_to_merge: impl AsRef<Path>,
    parent_bookmark_id: Option<u32>,
) -> Result<()> {
    trace!(
        "Merge the leaf (=PDF file) '{}' and add its bookmark",
        path_doc_to_merge.as_ref().display()
    );

    let mut doc_to_merge = Document::load(path_doc_to_merge.as_ref())?;

    let catalog_to_merge = doc_to_merge.catalog()?;
    let _ = catalog_to_merge
        .iter()
        .map(|(child_name, _child_object)| {
            let child_name = String::from_utf8(child_name.to_vec())?;

            if !ALLOWED_CATALOG_CHILDREN_FOR_INPUT_PDF.contains(&child_name) {
                return Err(anyhow!(
                    "The document contains the non supported \
                feature '{child_name}' among the Catalog children"
                ));
            }
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

    doc_to_merge.renumber_objects_with(main_doc.max_id + 1);

    let main_doc_pages_root_reference = main_doc.catalog()?.get(b"Pages")?.as_reference()?;
    let mut num_of_imported_object = 0;
    let first_page_id = {
        let pages = doc_to_merge.get_pages();
        *pages.get(&1).ok_or(anyhow!(
            "The document '{}' has 0 pages!",
            path_doc_to_merge.as_ref().display()
        ))?
    };

    for (object_id, mut object) in doc_to_merge.objects {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {}
            b"Pages" => {
                let pages_dict = object.as_dict_mut()?;

                if pages_dict.has(b"Parent") {
                    main_doc.objects.insert(object_id, object);
                } else {
                    pages_dict.set(b"Parent", main_doc_pages_root_reference);
                    main_doc
                        .objects
                        .insert(object_id, Object::Dictionary(pages_dict.clone()));

                    let main_doc_pages_root_dictionary = main_doc
                        .get_object_mut(main_doc_pages_root_reference)?
                        .as_dict_mut()?;

                    let pages_obj_reference_as_unit_vec = vec![Object::Reference(object_id)];

                    let imported_pages_count = pages_dict.get(b"Count")?.as_i64()?;

                    let actual_count = main_doc_pages_root_dictionary.get(b"Count")?.as_i64()?
                        + imported_pages_count;

                    main_doc_pages_root_dictionary.set(b"Count", Object::Integer(actual_count));
                    main_doc_pages_root_dictionary
                        .get_mut(b"Kids")?
                        .as_array_mut()?
                        .extend(pages_obj_reference_as_unit_vec);
                }
                num_of_imported_object += 1;
            }
            _ => {
                main_doc.objects.insert(object_id, object);
                num_of_imported_object += 1;
            }
        }
    }

    main_doc.max_id += num_of_imported_object;

    let name_doc_to_merge = path_doc_to_merge
        .as_ref()
        .file_name()
        .ok_or(anyhow!(
            "The given path '{}' does not contain a filename",
            path_doc_to_merge.as_ref().display()
        ))?
        .to_string_lossy()
        .to_string();

    let new_bookmark = Bookmark::new(
        name_doc_to_merge,
        BLACK_COLOR_RGB,
        DEFAULT_TEXT_FORMAT,
        first_page_id,
    );
    main_doc.add_bookmark(new_bookmark, parent_bookmark_id);

    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use super::*;
    use crate::utils;

    const TEST_DIR: &str = "dev-playground/test";

    #[test]
    fn merge_10_pages_leaf_in_main_doc() -> Result<()> {
        println!("Test 'merge_10_pages_leaf_in_main_doc'");
        let test_dir = get_virgin_test_dir("merge_10_pages_leaf_in_main_doc")?;

        let main_doc_name = "main_doc";

        let leaf_name = "leaf";
        let leaf_path = format!("{test_dir}/{leaf_name}");

        let mut main_doc = utils::get_basic_pdf_doc(main_doc_name, 15)?;
        let previous_max_id_main_doc = main_doc.max_id;
        let mut previous_pages_main_doc = main_doc.get_pages();
        let previous_len_main_doc = previous_pages_main_doc.len();

        let mut leaf_doc = utils::get_basic_pdf_doc(leaf_name, 10)?;

        let mut buffer = Vec::new();
        leaf_doc.save_modern(&mut buffer)?;
        std::fs::write(&leaf_path, buffer)?;

        let expected_page_ids_leaf_post_merge: BTreeMap<u32, (u32, u16)> = leaf_doc
            .get_pages()
            .iter()
            .map(|(&page_num, &page_id)| {
                let (id, version) = page_id;
                (
                    page_num + previous_len_main_doc as u32,
                    (id + previous_max_id_main_doc as u32, version),
                )
            })
            .collect();

        merge_from_leaf(&mut main_doc, leaf_path, None)?;

        previous_pages_main_doc.extend(expected_page_ids_leaf_post_merge.iter());

        let expected_pages_after_merge = previous_pages_main_doc;
        let pages_main_doc = main_doc.get_pages();
        assert_eq!(pages_main_doc, expected_pages_after_merge);

        Ok(())
    }

    fn get_virgin_test_dir(dir_name: impl AsRef<Path>) -> Result<String> {
        let dir_path = format!("{TEST_DIR}/{}", dir_name.as_ref().display());

        if std::fs::exists(&dir_path)? {
            std::fs::remove_dir_all(&dir_path)?;
        }

        std::fs::create_dir_all(&dir_path)?;
        Ok(dir_path)
    }

    #[test]
    fn merged_with_outline_and_save_modern_is_faulty_pdf() -> Result<()> {
        let test_dir = get_virgin_test_dir("merged_with_outline_and_save_modern_is_faulty_pdf")?;
        let target_dir_path = format!("{test_dir}/root_pdfs");
        let output_path = format!("{target_dir_path}.pdf");
        let compressed_output_path = format!("{target_dir_path}-compressed.pdf");
        let with_outlines = true;

        let minus_one = |n: u8| n - 1;
        utils::generate_fn_tree_with_levels(&target_dir_path, 3, 4, 2, 4, &minus_one)?;

        let mut main_doc = get_merged_tree_doc(target_dir_path, with_outlines)?;

        {
            let mut buffer = Vec::new();
            main_doc.save_modern(&mut buffer)?;
            std::fs::write(&output_path, buffer)?;

            main_doc.compress();

            let mut buffer = Vec::new();
            main_doc.save_modern(&mut buffer)?;
            std::fs::write(&compressed_output_path, buffer)?;
        }

        assert!(utils::validate_pdf(&output_path).is_err());
        assert!(utils::validate_pdf(&compressed_output_path).is_err());

        Ok(())
    }

    #[test]
    fn merged_without_outline_and_save_modern_is_faulty_pdf() -> Result<()> {
        let test_dir = get_virgin_test_dir("merged_without_outline_and_save_modern_is_faulty_pdf")?;
        let target_dir_path = format!("{test_dir}/root_pdfs");
        let output_path = format!("{target_dir_path}.pdf");
        let compressed_output_path = format!("{target_dir_path}-compressed.pdf");
        let with_outlines = false;

        let minus_one = |n: u8| n - 1;
        utils::generate_fn_tree_with_levels(&target_dir_path, 3, 4, 2, 4, &minus_one)?;

        let mut main_doc = get_merged_tree_doc(target_dir_path, with_outlines)?;

        {
            let mut buffer = Vec::new();
            main_doc.save_modern(&mut buffer)?;
            std::fs::write(&output_path, buffer)?;

            main_doc.compress();

            let mut buffer = Vec::new();
            main_doc.save_modern(&mut buffer)?;
            std::fs::write(&compressed_output_path, buffer)?;
        }

        assert!(utils::validate_pdf(&output_path).is_err());
        assert!(utils::validate_pdf(&compressed_output_path).is_err());

        Ok(())
    }

    #[test]
    fn merged_with_outline_and_save_is_ok() -> Result<()> {
        let test_dir = get_virgin_test_dir("merged_with_outline_and_save_is_ok")?;
        let target_dir_path = format!("{test_dir}/root_pdfs");
        let output_path = format!("{target_dir_path}.pdf");
        let compressed_output_path = format!("{target_dir_path}-compressed.pdf");
        let with_outlines = true;

        let minus_one = |n: u8| n - 1;
        utils::generate_fn_tree_with_levels(&target_dir_path, 3, 4, 2, 4, &minus_one)?;

        let mut main_doc = get_merged_tree_doc(target_dir_path, with_outlines)?;

        main_doc.save(&output_path)?;

        main_doc.compress();

        main_doc.save(&compressed_output_path)?;

        utils::validate_pdf(&output_path)?;
        utils::validate_pdf(&compressed_output_path)?;

        Ok(())
    }

    #[test]
    fn merged_without_outline_and_save_is_ok() -> Result<()> {
        let test_dir = get_virgin_test_dir("merged_without_outline_and_save_is_ok")?;
        let target_dir_path = format!("{test_dir}/root_pdfs");
        let output_path = format!("{target_dir_path}.pdf");
        let compressed_output_path = format!("{target_dir_path}-compressed.pdf");
        let with_outlines = false;

        let minus_one = |n: u8| n - 1;
        utils::generate_fn_tree_with_levels(&target_dir_path, 3, 4, 2, 4, &minus_one)?;

        let mut main_doc = get_merged_tree_doc(target_dir_path, with_outlines)?;

        main_doc.save(&output_path)?;

        main_doc.compress();

        main_doc.save(&compressed_output_path)?;

        utils::validate_pdf(&output_path)?;
        utils::validate_pdf(&compressed_output_path)?;

        Ok(())
    }
}
