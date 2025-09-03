use anyhow::{Result, anyhow};
use lopdf::{Document, Object, ObjectId, dictionary};
use std::path::Path;
use std::process::Command;

/// Uses `qpdf --check`, `pdfinfo` and `pdftotext -layout` to validate the PDF file.
pub fn validate_pdf(pdf_file_path: impl AsRef<Path>) -> Result<()> {
    let pdf_file_path = pdf_file_path.as_ref();

    // The processes are spawned in order of speed, from the slowest to the fastest.
    let qpdf_out = Command::new("qpdf")
        .arg("--check")
        .arg(pdf_file_path)
        .output()?;

    let pdftotext_out = Command::new("pdftotext")
        .arg("-layout")
        .arg(pdf_file_path)
        .output()?;

    let pdfinfo_out = Command::new("pdfinfo").arg(pdf_file_path).output()?;

    if !pdfinfo_out.status.success() {
        return Err(anyhow!(
            "`pdfinfo {}` returned with exit code {:?}: stdout [[{}]], stderr: [[{}]]",
            pdf_file_path.display(),
            pdfinfo_out.status.code(),
            str::from_utf8(&pdfinfo_out.stdout)?,
            str::from_utf8(&pdfinfo_out.stderr)?
        ));
    }

    if !pdfinfo_out.status.success() {
        return Err(anyhow!(
            "`pdftotext {}` returned with exit code {:?}: stdout [[{}]], stderr: [[{}]]",
            pdf_file_path.display(),
            pdftotext_out.status.code(),
            str::from_utf8(&pdftotext_out.stdout)?,
            str::from_utf8(&pdftotext_out.stderr)?
        ));
    }

    if !qpdf_out.status.success() {
        return Err(anyhow!(
            "`qpdf {}` returned with exit code {:?}: stdout [[{}]], stderr: [[{}]]",
            pdf_file_path.display(),
            qpdf_out.status.code(),
            str::from_utf8(&qpdf_out.stdout)?,
            str::from_utf8(&qpdf_out.stderr)?
        ));
    }

    Ok(())
}

/// Generates an a tree of directories of `num_levels` where the last level is pdf files.
/// The first generation has `num_siblings_this_level` children, and then each generation
/// applies recursively the function `siblings_fn` on the `num_siblings_this_level` input
/// to determine the future number of children. The parameter `constant_num_lateral_leaves`
/// tells how many pdf_files have to be created on each floor (but the last one), in
/// addition to the siblings.
///
/// Use with caution: if for example recursive_fn(n):=n, and we have no 'lateral leaves'
/// then we have an n-tree. An n-tree with L levels has sum(k=0, k=L) {n^k} nodes!
/// Furthermore if each pdf has p pages, this means p*(n^L) pdf pages in total!
pub fn generate_fn_tree_with_levels(
    root_pdfs: impl AsRef<Path>,
    num_levels: u8,
    num_siblings_this_level: u8,
    constant_num_lateral_leaves: u8,
    pages_per_pdf: u8,
    siblings_fn: &impl Fn(u8) -> u8,
) -> Result<()> {
    let root_pdfs = root_pdfs.as_ref();

    if std::fs::exists(root_pdfs)? {
        return Err(anyhow!(
            "The path '{}' exists already!",
            root_pdfs.display()
        ));
    }

    if num_levels == 0 {
        return Ok(());
    }

    if num_siblings_this_level == 0 {
        return Err(anyhow!(
            "The siblings to generate are {} and the levels still to create are {}",
            num_siblings_this_level,
            num_levels
        ));
    }

    std::fs::create_dir(root_pdfs)?;

    if num_levels == 1 {
        for sibling in 1..=num_siblings_this_level {
            let pdf_name = format!("pdf_doc{}.pdf", sibling);
            let pdf_path = format!("{}/{}", root_pdfs.display(), pdf_name);

            let mut pdf_doc = get_basic_pdf_doc(&pdf_name, pages_per_pdf)?;

            let mut buffer = Vec::new();
            pdf_doc.save_modern(&mut buffer)?;
            std::fs::write(pdf_path, &buffer)?;
        }
    } else {
        for sibling in 1..=num_siblings_this_level {
            let sibling_path = format!("{}/L{}S{}", root_pdfs.display(), num_levels, sibling);
            if let Err(err) = generate_fn_tree_with_levels(
                sibling_path,
                num_levels.saturating_sub(1),
                siblings_fn(num_siblings_this_level),
                constant_num_lateral_leaves,
                pages_per_pdf,
                siblings_fn,
            ) {
                // If encountering any error, the function tries to clean up after itself
                std::fs::remove_dir_all(root_pdfs)?;
                return Err(err);
            }
        }
        for lateral_leaf in 1..=constant_num_lateral_leaves {
            let pdf_name = format!("lateral_pdf_doc{}.pdf", lateral_leaf);
            let pdf_path = format!("{}/{}", root_pdfs.display(), pdf_name);

            let mut pdf_doc = get_basic_pdf_doc(&pdf_name, pages_per_pdf)?;

            let mut buffer = Vec::new();
            pdf_doc.save_modern(&mut buffer)?;
            std::fs::write(pdf_path, &buffer)?;
        }
    }

    Ok(())
}

pub fn get_catalog_children_names(doc: &Document) -> Result<Vec<String>> {
    let catalog = doc.catalog()?;

    let catalog_children_names = catalog
        .iter()
        .map(|(child_name, _child_object)| Ok(String::from_utf8(child_name.to_vec())?))
        .collect::<Result<Vec<_>>>()?;

    Ok(catalog_children_names)
}

/// Get a PDF file with minimal features
pub fn get_basic_pdf_doc(doc_name: &str, num_pages: u8) -> Result<Document> {
    if doc_name.contains('/') {
        return Err(anyhow!(
            "The document name provided contains a '/', not allowed!"
        ));
    }

    let mut doc = Document::with_version("1.7");

    let pages_root_id = doc.new_object_id();

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

    let pages_ids: Vec<_> = (1..=num_pages)
        .map(|page_number| {
            append_random_page_to_doc(page_number, num_pages, doc_name, &pages_root_id, &mut doc)
        })
        .collect::<Result<_>>()?;

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => pages_ids.iter().map(|&page_id| page_id.into()).collect::<Vec<_>>(),
        "Count" => num_pages,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    };

    doc.objects.insert(pages_root_id, Object::Dictionary(pages));
    doc.max_id += 1;

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_root_id,
    });

    doc.trailer.set("Root", catalog_id);

    Ok(doc)
}

fn append_random_page_to_doc(
    page_number: u8,
    total_num_pages: u8,
    doc_name: &str,
    pages_id: &ObjectId,
    doc: &mut Document,
) -> Result<ObjectId> {
    use lopdf::{
        Stream,
        content::{Content, Operation},
    };

    let page_title = format!("Page {page_number} of {total_num_pages}");
    let random_text = craft_random_text_of_len(20);

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

pub fn craft_random_text_of_len(char_length: usize) -> String {
    use rand::distr::{SampleString, StandardUniform};
    let random_string: String = StandardUniform.sample_string(&mut rand::rng(), char_length);
    random_string
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_doc_10_pages() -> Result<()> {
        let document = get_basic_pdf_doc("doc_name", 10)?;
        let len = document.get_pages().len();

        assert_eq!(len, 10);

        Ok(())
    }
}
