use std::iter::FromIterator;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use log::error;
use path_slash::PathExt;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::field_info::RELATIVE_FP_FIELD;
use crate::loader::Loader;
use crate::loader::LoaderResult;
use crate::loader::LoaderResultIterator;
use crate::worker::miner::{DEFAULT_ZONE_SEPARATION, Zone};

const HTML_ZONE_SEPARATION: u32 = 3;
const SEPARATOR_EL_SEPARATION: u32 = 0;

pub struct HtmlLoaderSelector {
    selector: Selector,
    field_name: Option<String>,
    attr_map: FxHashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct HtmlLoaderSelectorRaw {
    #[serde(default)]
    priority: u32,
    field_name: Option<String>,
    #[serde(default)]
    attr_map: FxHashMap<String, String>,
}

fn get_default_html_loader_selectors() -> FxHashMap<String, Option<HtmlLoaderSelectorRaw>> {
    FxHashMap::from_iter(vec![
        (
            "span[data-infisearch-link]".to_owned(),
            Some(HtmlLoaderSelectorRaw {
                priority: 0,
                field_name: None,
                attr_map: FxHashMap::from_iter(vec![
                    ("data-infisearch-link".to_owned(), "link".to_owned())
                ]),
            })
        ),
        (
            "title".to_owned(),
            Some(HtmlLoaderSelectorRaw {
                priority: 0,
                field_name: Some("title".to_owned()),
                attr_map: FxHashMap::default(),
            })
        ),
        (
            "h1".to_owned(),
            Some(HtmlLoaderSelectorRaw {
                priority: 0,
                field_name: Some("h1".to_owned()),
                attr_map: FxHashMap::default(),
            })
        ),
        (
            "body".to_owned(),
            Some(HtmlLoaderSelectorRaw {
                priority: 0,
                field_name: Some("body".to_owned()),
                attr_map: FxHashMap::default(),
            })
        ),
        (
            "meta[name=\"description\"],meta[name=\"keywords\"]".to_owned(),
            Some(HtmlLoaderSelectorRaw {
                priority: 0,
                field_name: None,
                attr_map: FxHashMap::from_iter(vec![
                    ("content".to_owned(), "body".to_owned())
                ]),
            })
        ),
        (
            "h2,h3,h4,h5,h6".to_owned(),
            Some(HtmlLoaderSelectorRaw {
                priority: 0,
                field_name: Some("heading".to_owned()),
                attr_map: FxHashMap::from_iter(vec![
                    ("id".to_owned(), "headingLink".to_owned())
                ]),
            })
        ),
    ])
}

fn get_default_exclude_selectors() -> Vec<String> {
    vec!["script,style,form,nav,[data-infisearch-ignore]".to_owned()]
}

#[derive(Serialize, Deserialize)]
pub struct HtmlLoaderOptionsRaw {
    #[serde(default)]
    selectors: FxHashMap<String, Option<HtmlLoaderSelectorRaw>>,
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
        let mut html_loader_options_raw: HtmlLoaderOptionsRaw =
            serde_json::from_value(config).expect("HtmlLoader options did not match schema!");

        // --------------------------------------------------------------
        // Merge/update the default selectors
        let mut selectors = get_default_html_loader_selectors();
        std::mem::swap(&mut selectors, &mut html_loader_options_raw.selectors);
        html_loader_options_raw.selectors.extend(selectors);
        // --------------------------------------------------------------
        
        let mut selectors: Vec<_> = html_loader_options_raw.selectors.iter()
            .filter_map(|(selector, opt)| opt.as_ref().map(|opt| (selector, opt)))    
            .collect();
        selectors.sort_by_key(|(_selector, opt)| opt.priority);
        selectors.reverse();

        let options = Arc::new(HtmlLoaderOptions {
            selectors: selectors
                .into_iter()
                .map(|(selector, opt)| HtmlLoaderSelector {
                    selector: Selector::parse(selector).expect("Invalid selector!"),
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
        panic!("Called deserialize for HtmlLoader")
    }
}

lazy_static! {
    /*
     Elements that should, in 99.9% of cases indicate a separation and likely directly contain text.
     Forcibly add a field separation in this case to prevent non-language-separated
     tokens from being joined together mistakenly.
     (e.g. "<td>a</td><td>b</td>" wrongly tokenized as "ab")
    */
    static ref SEPARATED_ELEMENTS: FxHashSet<&'static str> = FxHashSet::from_iter([
        "td", "th",
        "li", "dt", "dd",
        "h1", "h2", "h3", "h4", "h5", "h6",
        "hr", "br",
        "caption", "figcaption", "blockquote",
        "footer", "header", "main", "aside", "section", "article", "nav",
        "pre", "kbd", "p",
        "summary",
        "textarea",
        "label", "button", "legend", "option"
    ]);
}

impl HtmlLoaderResult {
    fn traverse_node<'a>(
        &'a self,
        node: ElementRef,
        // This controls whether to add a new Zone if the field_name is the same
        do_separate: &mut bool,
        field_texts: &mut Vec<Zone>,
        mut field_name_opt: Option<&'a String>,
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

                if let Some(selector_field_name) = &html_loader_selector.field_name {
                    field_name_opt = Some(selector_field_name);
                    break;
                }
            }
        }

        if let Some(field_name) = field_name_opt {
            // If there is no field name,
            // then nothing will be indexed and separator elements need not be handled

            let is_separator_el = SEPARATED_ELEMENTS.contains(node.value().name());

            // Tell the parent context to separate other text before this element too
            *do_separate = *do_separate || is_separator_el;

            for child in node.children() {
                if let Some(el_child) = ElementRef::wrap(child) {
                    self.traverse_node(el_child, do_separate, field_texts, field_name_opt);
                } else if let Some(text) = child.value().as_text() {
                    let last = unsafe { field_texts.last_mut().unwrap_unchecked() };
                    if last.field_name.as_str() == field_name.as_str() {
                        if *do_separate {
                            field_texts.push(Zone {
                                field_name: field_name.to_owned(),
                                field_text: text.to_string(),
                                separation: SEPARATOR_EL_SEPARATION,
                            });
                            *do_separate = false;
                        } else {
                            last.field_text += text;
                        }
                    } else {
                        field_texts.push(Zone {
                            field_name: field_name.to_owned(),
                            field_text: text.to_string(),
                            separation: HTML_ZONE_SEPARATION,
                        });
                        *do_separate = false;
                    }
                }
            }

            // Tell the parent context to separate other text after this element
            *do_separate = *do_separate || is_separator_el;
        } else {
            for child in node.children() {
                if let Some(el_child) = ElementRef::wrap(child) {
                    self.traverse_node(el_child, do_separate, field_texts, field_name_opt);
                }
            }
        }
    }
}

impl LoaderResult for HtmlLoaderResult {
    fn get_field_texts_and_path(mut self: Box<Self>) -> (Vec<Zone>, PathBuf) {
        let mut field_texts: Vec<Zone> = Vec::with_capacity(20);
        let mut document = Html::parse_document(&self.text);
        let mut do_separate = false;

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
                self.traverse_node(el_child, &mut do_separate, &mut field_texts, None);
            }
        }

        (field_texts, self.absolute_path)
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, path::PathBuf};

    use scraper::Selector;

    use crate::{loader::LoaderResult, worker::miner::{Zone, DEFAULT_ZONE_SEPARATION}, field_info::RELATIVE_FP_FIELD};

    use super::{
        HtmlLoaderOptions,
        HtmlLoaderResult,
        HtmlLoaderSelector,
        get_default_html_loader_selectors,
        get_default_exclude_selectors, HTML_ZONE_SEPARATION, SEPARATOR_EL_SEPARATION,
    };

    fn get_test_loader_options() -> HtmlLoaderOptions {
        HtmlLoaderOptions {
            selectors: get_default_html_loader_selectors()
                .into_iter()
                .filter_map(|(selector, opt)| opt.map(|opt| HtmlLoaderSelector {
                    selector: Selector::parse(&selector).expect("Invalid selector!"),
                    field_name: opt.field_name.clone(),
                    attr_map: opt.attr_map.clone(),
                }))
                .collect(),
            exclude_selectors: get_default_exclude_selectors()
                .iter()
                .map(|selector| Selector::parse(selector).expect("Invalid exclude selector!"))
                .collect(),
        }
    }

    #[test]
    fn test_separation() {
        let loader_result = Box::new(HtmlLoaderResult {
            link: String::new(),
            text: "text before".to_owned()
                + "<table>"
                + "<thead>"
                + "<tr><th>o<button>n</button>e</th><th>two</th></tr>"
                + "</thead><tbody>"
                + "<tr><th>three</td><td>four</td></tr>"
                + "<tr><th>five</td><td>si<button>x</button></td></tr>"
                + "</tbody>"
                + "</table>"
                + "text after"
                + "<h2><span>test</span> text</h2>",
            options: Arc::from(get_test_loader_options()),
            absolute_path: PathBuf::new(),
        });

        let (zones, _path) = loader_result.get_field_texts_and_path();
        assert_eq!(zones, vec![
            Zone {
                field_name: RELATIVE_FP_FIELD.to_owned(),
                field_text: "".to_owned(),
                separation: DEFAULT_ZONE_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "text before".to_owned(),
                separation: HTML_ZONE_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "o".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "n".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "e".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "two".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "three".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "four".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "five".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "si".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "x".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "body".to_owned(),
                field_text: "text after".to_owned(),
                separation: SEPARATOR_EL_SEPARATION,
            },
            Zone {
                field_name: "heading".to_owned(),
                field_text: "test text".to_owned(),
                separation: HTML_ZONE_SEPARATION,
            },
        ])
    }
}
