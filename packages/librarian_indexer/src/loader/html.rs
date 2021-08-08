use std::path::Path;

use path_slash::PathExt;
use scraper::ElementRef;
use scraper::Selector;
use scraper::Html;
use serde::Deserialize;

use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;

#[derive(Deserialize)]
pub struct HtmlLoaderOptions {

}

pub struct HtmlLoader {
    pub options: HtmlLoaderOptions,
}

struct HtmlLoaderResult {
    link: String,
    text: String,
}

impl Loader for HtmlLoader {
    fn try_index_file<'a> (&'a self, input_folder_path: &Path, path: &Path) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = path.extension() {
            if extension == "html" {
                return Some(Box::new(
                    vec![
                        Box::new(HtmlLoaderResult {
                            link: path.strip_prefix(input_folder_path).unwrap().to_slash().unwrap(),
                            text: std::fs::read_to_string(path).expect("Failed to read file!")
                        }) as Box<dyn LoaderResult + Send>
                    ].into_iter()
                ));
            }
        }

        None
    }
}

impl LoaderResult for HtmlLoaderResult {
    fn get_field_texts(&mut self) -> Vec<(&'static str, String)> {
        let mut field_texts: Vec<(&str, String)> = Vec::with_capacity(20);
        let document = Html::parse_document(&self.text);

        field_texts.push(("link", std::mem::take(&mut self.link)));

        if let Some(title) = document.select(&TITLE_SELECTOR).next() {
            field_texts.push(("title", title.text().collect()));
        }

        if let Some(body) = document.select(&BODY_SELECTOR).next() {
            traverse_node(body, &mut field_texts);
        }

        field_texts
    }
}

lazy_static! {
    static ref TITLE_SELECTOR: Selector = Selector::parse("title").unwrap();
    static ref BODY_SELECTOR: Selector = Selector::parse("body").unwrap();
}

fn traverse_node(node: ElementRef, field_texts: &mut Vec<(&str, String)>) {
    match node.value().name() {
        "h1"
        | "h2"
        | "h3"
        | "h4"
        | "h5"
        | "h6" => {
            field_texts.push(("heading", node.text().collect()));
        }
        _ => {
            if !node.has_children() {
                // field_texts.push(("body", node.text().collect()));
                return;
            }

            for child in node.children() {
                if let Some(el_child) = ElementRef::wrap(child) {
                    traverse_node(el_child, field_texts);
                } else {
                    if let Some(text) = child.value().as_text() {
                        if let Some(last) = field_texts.last_mut() {
                            if last.0 == "body" {
                                last.1 += text;
                            } else {
                                field_texts.push(("body", text.to_string()));
                            }
                        } else {
                            field_texts.push(("body", text.to_string()));
                        }
                    }
                }
            }
        }
    }
}
