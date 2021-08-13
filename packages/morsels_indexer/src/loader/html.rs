use std::path::Path;
use std::sync::Arc;

use path_slash::PathExt;
use scraper::ElementRef;
use scraper::Selector;
use scraper::Html;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use rustc_hash::FxHashMap;

use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;


pub struct HtmlLoaderSelector {
    selector: Selector,
    field_name: String,
    attr_map: FxHashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct HtmlLoaderSelectorRaw {
    selector: String,
    field_name: String,
    attr_map: FxHashMap<String, String>,
}

fn get_default_html_loader_selectors() -> Vec<HtmlLoaderSelectorRaw> {
    let mut heading_selector = HtmlLoaderSelectorRaw {
        selector: "h1,h2,h3,h4,h5,h6".to_owned(),
        field_name: "heading".to_owned(),
        attr_map: FxHashMap::default(),
    };
    heading_selector.attr_map.insert("id".to_owned(), "headingLink".to_owned());

    vec![
        HtmlLoaderSelectorRaw {
            selector: "title".to_owned(),
            field_name: "title".to_owned(),
            attr_map: FxHashMap::default(),
        },
        HtmlLoaderSelectorRaw {
            selector: "body".to_owned(),
            field_name: "body".to_owned(),
            attr_map: FxHashMap::default(),
        },
        heading_selector
    ]
}

#[derive(Serialize, Deserialize)]
pub struct HtmlLoaderOptionsRaw {
    #[serde(default = "get_default_html_loader_selectors")]
    selectors: Vec<HtmlLoaderSelectorRaw>,
    #[serde(default = "Vec::new")]
    exclude_selectors: Vec<String>
}

pub struct HtmlLoaderOptions {
    selectors: Vec<HtmlLoaderSelector>,
    exclude_selectors: Vec<Selector>
}

pub struct HtmlLoader {
    pub raw_options: HtmlLoaderOptionsRaw,
    pub options: Arc<HtmlLoaderOptions>,
}

struct HtmlLoaderResult {
    link: String,
    text: String,
    options: Arc<HtmlLoaderOptions>,
}

impl HtmlLoader {
    pub fn get_new_html_loader(config: serde_json::Value) -> Box<Self> {
        let html_loader_options_raw: HtmlLoaderOptionsRaw = serde_json::from_value(config)
            .expect("HtmlLoader options did not match schema!");

        let options = Arc::new(HtmlLoaderOptions {
            selectors: html_loader_options_raw.selectors
                .iter()
                .map(|opt| HtmlLoaderSelector {
                    selector: Selector::parse(&opt.selector).expect("Invalid selector!"),
                    field_name: opt.field_name.clone(),
                    attr_map: opt.attr_map.clone()
                })
                .collect(),
            exclude_selectors: html_loader_options_raw.exclude_selectors
                .iter()
                .map(|selector| Selector::parse(&selector).expect("Invalid exclude selector!"))
                .collect(),
        });

        Box::new(HtmlLoader {
            raw_options: html_loader_options_raw,
            options,
        })
    }
}

#[typetag::serde]
impl Loader for HtmlLoader {
    fn try_index_file<'a> (&'a self, _input_folder_path: &Path, absolute_path: &Path, relative_path: &Path) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = relative_path.extension() {
            if extension == "html" {
                return Some(Box::new(
                    std::iter::once(
                        Box::new(HtmlLoaderResult {
                            link: relative_path.to_slash().unwrap(),
                            text: std::fs::read_to_string(absolute_path).expect("Failed to read file!"),
                            options: self.options.clone(),
                        }) as Box<dyn LoaderResult + Send>
                    )
                ));
            }
        }

        None
    }

    fn get_name(&self) -> String {
        "HtmlLoader".to_owned()
    }
}

impl Serialize for HtmlLoader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.raw_options.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HtmlLoader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
        panic!("Called deserialize for CsvLoader")
    }
}

impl HtmlLoaderResult {
    fn traverse_node(&self, node: ElementRef, field_texts: &mut Vec<(String, String)>, field_name: Option<&String>) {
        for html_loader_selector in self.options.selectors.iter() {
            if html_loader_selector.selector.matches(&node) {
                let field_name = Some(&html_loader_selector.field_name);

                for (attr_name, attr_field_name) in html_loader_selector.attr_map.iter() {
                    if let Some(attr) = node.value().attr(attr_name) {
                        field_texts.push((attr_field_name.to_owned(), attr.to_owned()));
                    }
                }

                for child in node.children() {
                    if let Some(el_child) = ElementRef::wrap(child) {
                        // Traverse children elements with new field name
                        self.traverse_node(el_child, field_texts, field_name);
                    } else if let Some(text) = child.value().as_text() {
                        // Index children text nodes with new field name
                        if let Some(last) = field_texts.last_mut() {
                            if last.0 == html_loader_selector.field_name {
                                last.1 += text;
                            } else {
                                field_texts.push((html_loader_selector.field_name.to_owned(), text.to_string()));
                            }
                        } else {
                            field_texts.push((html_loader_selector.field_name.to_owned(), text.to_string()));
                        }
                    }
                }

                return;
            }
        }

        // No matching selector, use parent context
        for child in node.children() {
            if let Some(el_child) = ElementRef::wrap(child) {
                // Traverse children elements with parent field name
                self.traverse_node(el_child, field_texts, field_name);
            } else if let Some(text) = child.value().as_text() {
                if let Some(field_name) = field_name {
                    // Index children text nodes with parent field name
                    if let Some(last) = field_texts.last_mut() {
                        if last.0 == *field_name {
                            last.1 += text;
                        } else {
                            field_texts.push((field_name.to_owned(), text.to_string()));
                        }
                    } else {
                        field_texts.push((field_name.to_owned(), text.to_string()));
                    }
                }
            }
        }
    }
}

impl LoaderResult for HtmlLoaderResult {
    fn get_field_texts(&mut self) -> Vec<(String, String)> {
        let mut field_texts: Vec<(String, String)> = Vec::with_capacity(20);
        let mut document = Html::parse_document(&self.text);

        field_texts.push(("link".to_owned(), std::mem::take(&mut self.link)));

        for selector in self.options.exclude_selectors.iter() {
            let ids: Vec<_> = document.select(selector).map(|selected| selected.id()).collect();
            for id in ids {
                document.tree.get_mut(id).unwrap().detach();
            }
        }

        self.traverse_node(document.root_element(), &mut field_texts, None);

        field_texts
    }
}
