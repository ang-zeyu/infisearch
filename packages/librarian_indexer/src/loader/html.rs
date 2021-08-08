use std::path::Path;
use std::sync::Arc;

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
    #[serde(default = "Vec::new")]
    exclude_selectors: Vec<String>
}

pub struct HtmlLoader {
    pub options: HtmlLoaderOptions,

    pub exclude_selectors: Arc<Vec<Selector>>,
}

struct HtmlLoaderResult {
    link: String,
    text: String,
    exclude_selectors: Arc<Vec<Selector>>,
}

impl HtmlLoader {
    pub fn get_new_html_loader(config: serde_json::Value) -> Box<Self> {
        let html_loader_options: HtmlLoaderOptions = serde_json::from_value(config).expect("HtmlLoader options did not match schema!");

        let exclude_selectors = Arc::new(html_loader_options.exclude_selectors
            .iter()
            .map(|selector| Selector::parse(&selector).expect("Invalid exclude selector specified!"))
            .collect());

        Box::new(HtmlLoader {
            options: html_loader_options,
            exclude_selectors,
        })
    }
}

impl Loader for HtmlLoader {
    fn try_index_file<'a> (&'a self, input_folder_path: &Path, path: &Path) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = path.extension() {
            if extension == "html" {
                return Some(Box::new(
                    vec![
                        Box::new(HtmlLoaderResult {
                            link: path.strip_prefix(input_folder_path).unwrap().to_slash().unwrap(),
                            text: std::fs::read_to_string(path).expect("Failed to read file!"),
                            exclude_selectors: self.exclude_selectors.clone(),
                        }) as Box<dyn LoaderResult + Send>
                    ].into_iter()
                ));
            }
        }

        None
    }
}

impl LoaderResult for HtmlLoaderResult {
    fn get_field_texts(&mut self) -> Vec<(String, String)> {
        let mut field_texts: Vec<(String, String)> = Vec::with_capacity(20);
        let mut document = Html::parse_document(&self.text);

        field_texts.push(("link".to_owned(), std::mem::take(&mut self.link)));

        for selector in self.exclude_selectors.iter() {
            let ids: Vec<_> = document.select(selector).map(|selected| selected.id()).collect();
            for id in ids {
                document.tree.get_mut(id).unwrap().detach();
            }
        }

        if let Some(title) = document.select(&TITLE_SELECTOR).next() {
            field_texts.push(("title".to_owned(), title.text().collect()));
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

fn traverse_node(node: ElementRef, field_texts: &mut Vec<(String, String)>) {
    match node.value().name() {
        "h1"
        | "h2"
        | "h3"
        | "h4"
        | "h5"
        | "h6" => {
            field_texts.push(("heading".to_owned(), node.text().collect()));
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
                                field_texts.push(("body".to_owned(), text.to_string()));
                            }
                        } else {
                            field_texts.push(("body".to_owned(), text.to_string()));
                        }
                    }
                }
            }
        }
    }
}
