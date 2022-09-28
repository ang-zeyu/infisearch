use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use log::error;
use path_slash::PathExt;
use rustc_hash::FxHashMap;
use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::fieldinfo::RELATIVE_FP_FIELD;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;
use crate::worker::miner::{DEFAULT_ZONE_SEPARATION, Zone};

const HTML_ZONE_SEPARATION: u32 = 4;

pub struct HtmlLoaderSelector {
    selector: Selector,
    field_name: Option<String>,
    attr_map: FxHashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct HtmlLoaderSelectorRaw {
    selector: String,
    field_name: Option<String>,
    attr_map: FxHashMap<String, String>,
}

fn get_default_html_loader_selectors() -> Vec<HtmlLoaderSelectorRaw> {
    let mut heading_selector = HtmlLoaderSelectorRaw {
        selector: "h2,h3,h4,h5,h6".to_owned(),
        field_name: Some("heading".to_owned()),
        attr_map: FxHashMap::default(),
    };
    heading_selector.attr_map.insert("id".to_owned(), "headingLink".to_owned());

    vec![
        HtmlLoaderSelectorRaw {
            selector: "span[data-morsels-link]".to_owned(),
            field_name: None,
            attr_map: {
                let mut map = FxHashMap::default();
                map.insert("data-morsels-link".to_owned(), "link".to_owned());
                map
            },
        },
        HtmlLoaderSelectorRaw {
            selector: "title".to_owned(),
            field_name: Some("title".to_owned()),
            attr_map: FxHashMap::default(),
        },
        HtmlLoaderSelectorRaw {
            selector: "h1".to_owned(),
            field_name: Some("h1".to_owned()),
            attr_map: FxHashMap::default(),
        },
        HtmlLoaderSelectorRaw {
            selector: "body".to_owned(),
            field_name: Some("body".to_owned()),
            attr_map: FxHashMap::default(),
        },
        heading_selector,
    ]
}

fn get_default_exclude_selectors() -> Vec<String> {
    vec!["script,style,pre".to_owned()]
}

#[derive(Serialize, Deserialize)]
pub struct HtmlLoaderOptionsRaw {
    #[serde(default = "get_default_html_loader_selectors")]
    selectors: Vec<HtmlLoaderSelectorRaw>,
    #[serde(default = "get_default_exclude_selectors")]
    exclude_selectors: Vec<String>,
}

pub struct HtmlLoaderOptions {
    selectors: Vec<HtmlLoaderSelector>,
    exclude_selectors: Vec<Selector>,
}

pub struct HtmlLoader {
    pub raw_options: HtmlLoaderOptionsRaw,
    pub options: Arc<HtmlLoaderOptions>,
}

struct HtmlLoaderResult {
    link: String,
    text: String,
    options: Arc<HtmlLoaderOptions>,
    absolute_path: PathBuf,
}

impl HtmlLoader {
    pub fn get_new_html_loader(config: serde_json::Value) -> Box<Self> {
        let html_loader_options_raw: HtmlLoaderOptionsRaw =
            serde_json::from_value(config).expect("HtmlLoader options did not match schema!");

        let options = Arc::new(HtmlLoaderOptions {
            selectors: html_loader_options_raw
                .selectors
                .iter()
                .map(|opt| HtmlLoaderSelector {
                    selector: Selector::parse(&opt.selector).expect("Invalid selector!"),
                    field_name: opt.field_name.clone(),
                    attr_map: opt.attr_map.clone(),
                })
                .collect(),
            exclude_selectors: html_loader_options_raw
                .exclude_selectors
                .iter()
                .map(|selector| Selector::parse(selector).expect("Invalid exclude selector!"))
                .collect(),
        });

        Box::new(HtmlLoader { raw_options: html_loader_options_raw, options })
    }
}

#[typetag::serde]
impl Loader for HtmlLoader {
    fn try_index_file<'a>(
        &'a self,
        absolute_path: &Path,
        relative_path: &Path,
    ) -> Option<LoaderResultIterator<'a>> {
        if let Some(extension) = relative_path.extension() {
            if extension == "html" {
                let absolute_path_as_buf = PathBuf::from(absolute_path);

                if let Some(relative_path) = relative_path.to_slash() {
                    return Some(Box::new(std::iter::once(Box::new(HtmlLoaderResult {
                        link: relative_path.into_owned(),
                        text: std::fs::read_to_string(absolute_path).expect("Failed to read file!"),
                        options: self.options.clone(),
                        absolute_path: absolute_path_as_buf,
                    }) as Box<dyn LoaderResult + Send>)));
                } else {
                    error!("Unable to index {} containing non-unicode characters", relative_path.to_slash_lossy());
                }
            }
        }

        None
    }

    fn get_name(&self) -> String {
        "HtmlLoader".to_owned()
    }
}

impl Serialize for HtmlLoader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.raw_options.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HtmlLoader {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("Called deserialize for CsvLoader")
    }
}

impl HtmlLoaderResult {
    fn traverse_node(
        &self,
        node: ElementRef,
        field_texts: &mut Vec<Zone>,
        field_name: Option<&String>,
    ) {
        for html_loader_selector in self.options.selectors.iter() {
            if html_loader_selector.selector.matches(&node) {
                for (attr_name, attr_field_name) in html_loader_selector.attr_map.iter() {
                    if let Some(attr) = node.value().attr(attr_name) {
                        field_texts.push(Zone {
                            field_name: attr_field_name.to_owned(),
                            field_text: attr.to_owned(),
                            separation: 2,
                        });
                    }
                }

                if let Some(field_name) = &html_loader_selector.field_name {
                    for child in node.children() {
                        if let Some(el_child) = ElementRef::wrap(child) {
                            // Traverse children elements with new field name
                            self.traverse_node(el_child, field_texts, Some(field_name));
                        } else if let Some(text) = child.value().as_text() {
                            // Index children text nodes with new field name
                            add_text_to_field(field_texts, field_name, text);
                        }
                    }
    
                    return;
                }
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
                    add_text_to_field(field_texts, field_name, text);
                }
            }
        }
    }
}

#[inline(always)]
fn add_text_to_field(field_texts: &mut Vec<Zone>, field_name: &String, text: &scraper::node::Text) {
    if let Some(last) = field_texts.last_mut() {
        if last.field_name == *field_name {
            last.field_text += text;
        } else {
            field_texts.push(Zone {
                field_name: field_name.to_owned(),
                field_text: text.to_string(),
                separation: HTML_ZONE_SEPARATION,
            });
        }
    } else {
        field_texts.push(Zone {
            field_name: field_name.to_owned(),
            field_text: text.to_string(),
            separation: HTML_ZONE_SEPARATION,
        });
    }
}

impl LoaderResult for HtmlLoaderResult {
    fn get_field_texts_and_path(mut self: Box<Self>) -> (Vec<Zone>, PathBuf) {
        let mut field_texts: Vec<Zone> = Vec::with_capacity(20);
        let mut document = Html::parse_document(&self.text);

        field_texts.push(Zone {
            field_name: RELATIVE_FP_FIELD.to_owned(),
            field_text: std::mem::take(&mut self.link),
            separation: DEFAULT_ZONE_SEPARATION,
        });

        for selector in self.options.exclude_selectors.iter() {
            let ids: Vec<_> = document.select(selector).map(|selected| selected.id()).collect();
            for id in ids {
                document.tree.get_mut(id).unwrap().detach();
            }
        }

        for child in document.tree.root().children() {
            if let Some(el_child) = ElementRef::wrap(child) {
                self.traverse_node(el_child, &mut field_texts, None);
            }
        }

        (field_texts, self.absolute_path)
    }
}
