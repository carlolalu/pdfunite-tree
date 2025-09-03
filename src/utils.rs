use anyhow::{Result, anyhow};
use lopdf::{Document, Object, ObjectId, dictionary};

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

fn craft_random_text_of_len(char_length: usize) -> String {
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
