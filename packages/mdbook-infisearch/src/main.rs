extern crate mdbook;

use std::fs::{self, File};
use std::io::Write;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::Error;
use clap::App;
use clap::Arg;
use clap::SubCommand;
use mdbook::book::Book;
use mdbook::book::BookItem;
use mdbook::preprocess::CmdPreprocessor;
use mdbook::preprocess::Preprocessor;
use mdbook::preprocess::PreprocessorContext;
use mdbook::renderer::RenderContext;
use infisearch::assets;
use infisearch::indexer::Indexer;
use infisearch::indexer::input_config::InfiConfig;
use toml::value::Value::{self, String as TomlString};
use serde_json::Value as JsonValue;
use walkdir::WalkDir;

const DEFAULT_CONFIG: &'static str = include_str!("../default_infi_search.json");

const CONFIG_KEY: &'static str = "output.infisearch.config";

const MARK_MIN_JS: &[u8] = include_bytes!("../mark.min.js");

pub fn make_app() -> App<'static, 'static> {
    App::new("InfiSearch").about("InfiSearch preprocessor + renderer for mdbook").subcommand(
        SubCommand::with_name("supports")
            .arg(Arg::with_name("renderer").required(true))
            .about("Check whether a renderer is supported by this preprocessor"),
    )
}

fn main() {
    let matches = make_app().get_matches();

    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();

    if let Ok(ctx) = RenderContext::from_json(&*buf) {
        let html_renderer_path = ctx.destination.join("../html");
        let assets_output_dir = html_renderer_path.join("infisearch_assets");
        fs::create_dir_all(&assets_output_dir)
            .expect("mdbook-infisearch: Failed to create assets directory.");

        // ---------------------------------
        // Copy mark.min.js
        let mut mark_js = File::create((&assets_output_dir).join(Path::new("mark.min.js")))
            .expect("mdbook-infisearch: Failed to open asset write handler");
        mark_js.write_all(MARK_MIN_JS).expect("mdbook-infisearch: Failed to copy search-ui asset (mark.min.js)!");
        // ---------------------------------

        let input_folder_path = html_renderer_path.clone();
        let output_folder_path = input_folder_path.join("infisearch_output");
        let is_incremental = ctx.config.get("output.html.livereload-url").is_some();

        let config = setup_config_file(&ctx.root, ctx.config.get(CONFIG_KEY));

        let mut indexer = Indexer::new(
            &input_folder_path,
            &output_folder_path,
            InfiConfig::new(config),
            is_incremental,
            false,
            false,
        );

        for entry in WalkDir::new(input_folder_path.clone()) {
            match entry {
                Ok(dir_entry) => {
                    if !dir_entry.file_type().is_file() {
                        continue;
                    }
    
                    let path = dir_entry.path();
                    let relative_path = path.strip_prefix(&input_folder_path).unwrap();
    
                    indexer.index_file(path, relative_path);
                }
                Err(e) => {
                    panic!("Error processing entry. {}", e)
                }
            }
        }

        indexer.finish_writing_docs(None);

        assets::write_infisearch_assets(&assets_output_dir);
    } else {
        let infisearch_preprocessor = InfiSearch;

        if let Some(sub_args) = matches.subcommand_matches("supports") {
            let renderer = sub_args.value_of("renderer").expect("Required argument");

            if renderer != "html" {
                std::process::exit(1);
            }
        } else {
            let (ctx, book) = CmdPreprocessor::parse_input(&*buf).expect("mdbook-infisearch: Preprocess JSON parsing failed");
            let processed_book = infisearch_preprocessor.run(&ctx, book).expect("mdbook-infisearch: Preprocess processing failed");
            serde_json::to_writer(io::stdout(), &processed_book).unwrap();
        }

        std::process::exit(0);
    }
}


fn setup_config_file(root: &Path, config: Option<&Value>) -> String {
    if let Some(config_path) = get_config_file_path(root, config) {
        if !config_path.exists() || !config_path.is_file() {
            fs::write(&config_path, DEFAULT_CONFIG).expect("Failed to write default InfiSearch configuration");
        }

        std::fs::read_to_string(&config_path).expect("invalid InfiSearch configuration file")
    } else {
        String::from(DEFAULT_CONFIG)
    }
}

fn get_config_file_path(root: &Path, config: Option<&Value>) -> Option<PathBuf> {
    if let Some(TomlString(config_file_path)) = config {
        Some(root.join(config_file_path))
    } else {
        None
    }
}

// Preprocessor for adding input search box
pub struct InfiSearch;

static INPUT_EL: &str = "\n<input
    type=\"search\"
    id=\"infi-search\"
    placeholder=\"Search this book ...\"
/>\n\n
<span style=\"font-weight: 600;\"><!--preload weight 600--></span>\n\n
<div id=\"infisearch-mdbook-target\"></div>\n\n";

static STYLES: &str = include_str!("infisearch.css");

fn get_css_el(base_url: &str) -> String {
    format!(
        "<link rel=\"stylesheet\" href=\"{}infisearch_assets/search-ui-light.css\">\n\n<style>{}</style>\n",
        base_url,
        STYLES,
    )
}

fn get_script_els(ctx: &PreprocessorContext, base_url: &str) -> String {
    let mode = if let Some(TomlString(mode)) = ctx.config.get("output.infisearch.mode") {
        if mode == "query_param" {
            // Documentation specific, do not use!
            // For demoing the different modes only
            "(function () {
                // This IIFE is documentation specific, for demoing the different modes.
                // It would be the string mode (e.g. 'target') normally
                const params = new URLSearchParams(window.location.search);
                return params.get('mode') || 'target';
            })()".to_owned()
        } else {
            let valid_modes = vec!["auto", "dropdown", "fullscreen", "target"];
            if valid_modes.into_iter().any(|valid_mode| valid_mode == mode) {
                format!("'{}'", mode)
            } else {
                "'target'".to_owned()
            }
        }
    } else {
        "'target'".to_owned()
    };

    let config = setup_config_file(&ctx.root, ctx.config.get(CONFIG_KEY));
    let config_as_value: JsonValue = serde_json::from_str(&config)
        .expect("unexpected error parsing search config file");
    let lang = if let Some(lang_config) = config_as_value.get("lang_config") {
        if let Some(serde_json::Value::String(lang)) = lang_config.get("lang") {
            lang
        } else {
            "ascii"
        }
    } else {
        "ascii"
    };

    let infisearch_js = include_str!("infisearch.js");
    format!(
"\n
<script src=\"{}infisearch_assets/search-ui.{}.bundle.js\" type=\"text/javascript\" charset=\"utf-8\"></script>
<script src=\"{}infisearch_assets/mark.min.js\" type=\"text/javascript\" charset=\"utf-8\"></script>\n
<script>
const base_url = '{}';
const mode = {};
{}
</script>",
        base_url, lang,
        base_url,
        base_url,
        mode,
        infisearch_js,
    )
}

fn get_part_title_el(part_title: &str) -> String {
    format!("\n\n<span data-infisearch-part-title=\"{}\"></span>\n", part_title)
}


impl Preprocessor for InfiSearch {
    fn name(&self) -> &str {
        "infisearch"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        if let Some(nop_cfg) = ctx.config.get_preprocessor("infisearch") {
            if nop_cfg.contains_key("blow-up") {
                anyhow::bail!("Boom!!1!");
            }
        }

        let site_url = if let Some(TomlString(site_url)) = ctx.config.get("output.html.site-url") {
            site_url
        } else {
            "/"
        };

        let init_infisearch_el = get_script_els(ctx, site_url);

        let mut total_len: u64 = 0;

        let mut current_part_title: Option<String> = None;
        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(ch) = item {
                total_len += ch.content.len() as u64;

                let part_title = if let Some(current_part_title) = &current_part_title {
                    get_part_title_el(current_part_title)
                } else {
                    "".to_owned()
                };

                ch.content = get_css_el(site_url)
                    + INPUT_EL
                    + ch.content.as_str()
                    + init_infisearch_el.as_str()
                    + part_title.as_str();
            } else if let BookItem::PartTitle(part_title) = item {
                current_part_title = Some(part_title.to_owned());
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}
