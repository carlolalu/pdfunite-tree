pub mod utils;

use anyhow::{Result, anyhow};
use clap::Parser;
use lazy_static::lazy_static;
use log::{info, trace};
use lopdf::{Document, Object, dictionary};
use std::path::Path;

const MAX_DEPTH_PDF_TREE: u8 = 6;
const DEFAULT_OUTPUT_SUFFIX: &str = "-united.pdf";

lazy_static! {
    static ref ALLOWED_CATALOG_CHILDREN_FOR_INPUT_PDF: Vec<String> =
        ["Type", "Version", "Pages", "PageMode"]
            .map(|not_owned| not_owned.to_string())
            .into_iter()
            .collect();
}

/// Merge together all the PDFs in a directory and its subfolders (max X levels) into a single document
/// provided with a ToC (Table of Contents) reflecting the structure of tree of the directory and its descendants.
/// The tree of old PDF files will not be modified.
///
/// Assumptions on the pdf tree:
/// 1. The tree has not more than 4 levels (the root is considered level 0).
/// 2. All the files in the root directory and its subdirectories are PDFs and their names are UT8-encoded.
/// 3. No PDF file in the directory and its subdirectory is already equipped with a ToC, a Destination tree or other special features.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Directory containing the pdfs
    input_directory: String,
    /// Output path (must not be contained in the input-directory)
    #[arg(short = 'o')]
    output_path: Option<String>,
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
    };

    let output_path = cli.output_path.unwrap_or(format!(
        "{}{DEFAULT_OUTPUT_SUFFIX}",
        target_dir_path.display()
    ));
    let output_path = Path::new(&output_path);

    if output_path.starts_with(target_dir_path) {
        return Err(anyhow!(
            "The output file cannot be a descendant of the input directory: \
            '{}' is a descendant of '{}'",
            output_path.display(),
            target_dir_path.display()
        ));
    }

    merge_tree(target_dir_path, output_path)?;

    Ok(())
}

fn merge_tree(target_dir_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<()> {
    let target_dir_path = target_dir_path.as_ref();
    let output_path = output_path.as_ref();

    let mut main_doc = Document::with_version("1.7");

    info!("Initialising main document");
    initialise_doc_with_null_pages(&mut main_doc)?;

    info!("Start the merging process");
    merge_from_internal_node(&mut main_doc, target_dir_path, 0)?;

    if std::fs::exists(output_path)? {
        return Err(anyhow!(
            "A file '{}' is already present",
            output_path.display()
        ));
    } else {
        let mut buffer = Vec::new();
        main_doc.save_modern(&mut buffer)?;
        std::fs::write(output_path, buffer)?;
    }

    Ok(())
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
) -> Result<()> {
    trace!(
        "Merge the node (=symlink or directory) '{}'",
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
    entries.sort_by_key(|dir_entry| dir_entry.path());

    for entry in entries {
        let file_type = entry.file_type()?;

        if file_type.is_file() {
            merge_from_leaf(main_doc, entry.path())?;
        } else {
            merge_from_internal_node(main_doc, entry.path(), parent_level + 1)?;
        }
    }

    Ok(())
}

fn merge_from_leaf(main_doc: &mut Document, path_doc_to_merge: impl AsRef<Path>) -> Result<()> {
    trace!(
        "Merge the leaf (=PDF file) '{}'",
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

    trace!("Import objects different from 'Catalog'");

    let main_doc_pages_root_reference = main_doc.catalog()?.get(b"Pages")?.as_reference()?;
    let mut num_of_imported_object = 0;

    for (object_id, mut object) in doc_to_merge.objects {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {}
            b"Pages" => {
                let pages_dict = object.as_dict_mut()?;

                if pages_dict.has(b"Parent") {
                    main_doc.objects.insert(object_id, object);
                } else {
                    trace!(r##"The "Pages" root of the leaf has been found"##);

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

        merge_from_leaf(&mut main_doc, leaf_path)?;

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
}
