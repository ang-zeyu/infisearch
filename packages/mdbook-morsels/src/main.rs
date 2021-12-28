extern crate mdbook;

use std::fs::{self, File};
use std::io::Write;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Error;
use clap::App;
use clap::Arg;
use clap::SubCommand;
use include_dir::{include_dir, Dir};
use mdbook::book::Book;
use mdbook::book::BookItem;
use mdbook::preprocess::CmdPreprocessor;
use mdbook::preprocess::Preprocessor;
use mdbook::preprocess::PreprocessorContext;
use mdbook::renderer::RenderContext;
use toml::value::Value::{self, Boolean as TomlBoolean, String as TomlString};
use serde_json::{Value as JsonValue, json};

const SEARCH_UI_DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/search-ui-dist");

const MARK_MIN_JS: &[u8] = include_bytes!("../mark.min.js");

pub fn make_app() -> App<'static, 'static> {
    App::new("morsels").about("Morsels preprocessor + renderer for mdbook").subcommand(
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

        // ---------------------------------
        // Copy assets
        let assets_output_dir = html_renderer_path.join("morsels_assets");
        fs::create_dir_all(&assets_output_dir)
            .expect("mdbook-morsels: Failed to create assets output directory");
        for file in SEARCH_UI_DIST.files() {
            let mut output_file = File::create((&assets_output_dir).join(file.path()))
                .expect("mdbook-morsels: Failed to open asset write handler");
            output_file.write_all(file.contents()).expect("mdbook-morsels: Failed to copy search-ui assets!");
        }
        let mut mark_js = File::create((&assets_output_dir).join(Path::new("mark.min.js")))
            .expect("mdbook-morsels: Failed to open asset write handler");
        mark_js.write_all(MARK_MIN_JS).expect("mdbook-morsels: Failed to copy search-ui asset (mark.min.js)!");
        // ---------------------------------

        let morsels_config_path = get_config_file_path(&ctx.root, ctx.config.get("output.morsels.config"));

        let mut command = Command::new("morsels");
        command
            .current_dir(html_renderer_path)
            .args(&["./", "./morsels_output"])
            .arg("-c")
            .arg(morsels_config_path);

        if let Some(_livereload_url) = ctx.config.get("output.html.livereload-url") {
            command.arg("--dynamic");
        }

        let output = command.output().expect("mdbook-morsels: failed to execute indexer process");
        let mut log_file = File::create(ctx.destination.join(Path::new("morsels_indexer_log.txt"))).unwrap();
        log_file.write_all("\n".as_bytes()).unwrap();
        log_file.write_all(&output.stdout).unwrap();
        log_file.write_all("\n".as_bytes()).unwrap();
        log_file.write_all(&output.stderr).unwrap();
        log_file.flush().unwrap();
    } else {
        let morsels_preprocessor = Morsels;

        if let Some(sub_args) = matches.subcommand_matches("supports") {
            let renderer = sub_args.value_of("renderer").expect("Required argument");

            if renderer == "html" {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        } else {
            let (ctx, book) = CmdPreprocessor::parse_input(&*buf).expect("mdbook-morsels: Preprocess JSON parsing failed");
            let processed_book = morsels_preprocessor.run(&ctx, book).expect("mdbook-morsels: Preprocess processing failed");
            serde_json::to_writer(io::stdout(), &processed_book).unwrap();
            std::process::exit(0);
        }
    }
}

fn setup_config_file(ctx: &PreprocessorContext, total_len: u64) -> std::path::PathBuf {
    let morsels_config_path = get_config_file_path(&ctx.root, ctx.config.get("output.morsels.config"));

    if !morsels_config_path.exists() || !morsels_config_path.is_file() {
        let mut init_config_command = Command::new("morsels");
        init_config_command.current_dir(ctx.root.clone()).args(&["./", "./morsels_output", "--init"]);
        init_config_command.arg("-c");
        init_config_command.arg(&morsels_config_path);
        init_config_command
            .output()
            .expect("mdbook-morsels: failed to create default configuration file");

        let config = fs::read_to_string(&morsels_config_path).unwrap();
        fs::write(
            &morsels_config_path,
            config.replace("\"exclude\": [", "\"exclude\": [\n      \"index.html\",\n      \"print.html\", \n      \"404.html\",")
                .replace("script,style", "script,style,#sidebar,#menu-bar"),
        )
        .unwrap();
    }

    let scaling_config = ctx.config.get("output.morsels.scaling");
    let do_scale = scaling_config.is_none()
        || (scaling_config.unwrap().is_bool() && scaling_config.unwrap().as_bool().unwrap());
    if do_scale  {
        auto_scale_config(
            &morsels_config_path,
            total_len,
            ctx.config.get("debug").unwrap_or_else(|| &Value::Boolean(false)).as_bool().unwrap_or_else(|| false),
        );
    }

    morsels_config_path
}

fn auto_scale_config(morsels_config_path: &PathBuf, total_len: u64, debug: bool) {
    let config = fs::read_to_string(morsels_config_path).unwrap();
    let mut config_as_value: JsonValue = serde_json::from_str(&config).expect("unexpected error parsing search config file");
    let indexing_config = config_as_value.get_mut("indexing_config");
    if indexing_config.is_none() {
        config_as_value.as_object_mut().unwrap().insert("indexing_config".to_owned(), json!("{}"));
    }
    let indexing_config = config_as_value
        .get_mut("indexing_config")
        .unwrap()
        .as_object_mut()
        .expect("indexing_config is not an object");

    if debug {
        indexing_config.insert(
            "_total_len".to_owned(),
            json!(format!("This is debugging metadata for the mdbook plugin. Total len: {}", total_len))
        );
    }
    
    const BOUNDARY_1_END: u64   = 10000000;
    // 10MB
    const BOUNDARY_2_START: u64 = 10000001;
    // 100MB
    const BOUNDARY_2_END: u64   = 100000000;
    match total_len {
        0..=BOUNDARY_2_END => {
            indexing_config.insert("num_docs_per_block".to_owned(), json!(1000));
            indexing_config.insert("pl_limit".to_owned(), json!(2097151));
            indexing_config.insert("pl_cache_threshold".to_owned(), json!(0));
        },
        _ => {
            indexing_config.insert("num_docs_per_block".to_owned(), json!(1000));
            indexing_config.insert("pl_limit".to_owned(), json!(16383));
            indexing_config.insert("pl_cache_threshold".to_owned(), json!(1048576));
        }
    }
    let fields_config = config_as_value.get_mut("fields_config");
    if fields_config.is_none() {
        config_as_value.as_object_mut().unwrap().insert("fields_config".to_owned(), json!("{}"));
    }
    let fields_config = config_as_value
        .get_mut("fields_config")
        .expect("fields_config is missing!")
        .as_object_mut()
        .expect("fields_config is not an object");
    let set_field_do_store = |fields: &mut Vec<JsonValue>, name: &str, val: bool| {
        let field = fields
            .iter_mut()
            .find(|val|
                val.as_object()
                    .expect("encountered non object in fields_config.fields")
                    .get("name")
                    .expect("fields_config.fields object missing key \"name\"")
                    .as_str()
                    .expect("fields_config.fields.name is not a string")
                == name
            )
            .expect(&("fields_config.fields is missing ".to_owned() + name))
            .as_object_mut()
            .unwrap();
        field.insert("do_store".to_owned(), json!(val));
    };
    match total_len {
        0..=BOUNDARY_1_END => {
            fields_config.insert("field_store_block_size".to_owned(), json!(250));
            fields_config.insert("cache_all_field_stores".to_owned(), json!(true));
        },
        BOUNDARY_2_START..=BOUNDARY_2_END => {
            fields_config.insert("field_store_block_size".to_owned(), json!(3));
            fields_config.insert("cache_all_field_stores".to_owned(), json!(false));
        },
        _ => {
            fields_config.insert("field_store_block_size".to_owned(), json!(1));
            fields_config.insert("cache_all_field_stores".to_owned(), json!(false));
        }
    }

    let mut fields = fields_config
        .get_mut("fields")
        .expect("fields_config.fields is missing!")
        .as_array_mut()
        .expect("fields_config.fields is not an array");

    // All scaling levels prefer json stores for now
    set_field_do_store(&mut fields, "title", true);
    set_field_do_store(&mut fields, "heading", true);
    set_field_do_store(&mut fields, "headingLink", true);
    set_field_do_store(&mut fields, "body", true);

    fs::write(
        &morsels_config_path,
        serde_json::to_string_pretty(&config_as_value).unwrap(),
    )
    .unwrap();
}

fn get_config_file_path(root: &PathBuf, config: Option<&Value>) -> std::path::PathBuf {
    let morsels_config_path = if let Some(TomlString(morsels_config_file_path)) = config {
        root.join(morsels_config_file_path)
    } else {
        root.join("morsels_config.json")
    };
    morsels_config_path
}

// Preprocessor for adding input search box
pub struct Morsels;

static INPUT_EL: &str = "\n<input
    type=\"text\"
    id=\"morsels-search\"
    placeholder=\"Search\"
/>\n\n
<span style=\"font-weight: 600;\"><!--preload weight 600--></span>\n\n
<div class=\"morsels-root\" id=\"morsels-mdbook-target\"></div>\n\n";

static STYLES: &str = r#"
<style>
.morsels-root {
    --morsels-border: 3px solid var(--table-header-bg);
    --morsels-fg: var(--fg);
    --morsels-bg: var(--bg);
    --morsels-item-border-radius: 10px;
    --morsels-item-sub-border:  1px solid var(--table-border-color);
    --morsels-dropdown-input-separator-bg: var(--table-header-bg);
    --morsels-title-bg: var(--table-header-bg);
    --morsels-title-hover-fg: var(--bg);
    --morsels-title-hover-bg: var(--fg);
    --morsels-heading-bg: var(--table-alternate-bg);
    --morsels-heading-hover-bg: var(--table-header-bg);
    --morsels-body-hover-bg: var(--table-alternate-bg);
    --morsels-highlight: var(--search-mark-bg);
    --morsels-error-fg: var(--fg);
    --morsels-fine-print-fg: var(--fg);
    --morsels-loading-bg: var(--fg);
    --morsels-scrollbar-bg: var(--sidebar-bg);
    --morsels-scrollbar-thumb-bg: var(--sidebar-non-existant);
    --morsels-fullscreen-box-shadow: none;
    --morsels-fullscreen-header-bg: var(--sidebar-bg);
    --morsels-fullscreen-input-border: 2px solid var(--searchbar-border-color);
    --morsels-fullscreen-input-focus-border: 2px solid var(--searchbar-border-color);
    --morsels-fullscreen-input-focus-box-shadow: 0 0 5px var(--searchbar-shadow-color);
    --morsels-fullscreen-header-close-fg: var(--sidebar-fg);
    --morsels-fullscreen-header-close-bg: var(--sidebar-non-existant);
    --morsels-fullscreen-header-close-hover-bg: var(--theme-popup-border);
    --morsels-fullscreen-header-close-hover-fg: var(--sidebar-spacer);
    --morsels-fullscreen-header-close-active-bg: var(--theme-popup-border);
    --morsels-fullscreen-header-close-active-fg: var(--sidebar-spacer);
}

#morsels-search {
    width: 100%;
    border-radius: 6px;
    box-sizing: border-box;
    font-size: 16px;
    padding: 0.5em 0.75em;
    border: 1px solid var(--searchbar-border-color);
    background: var(--searchbar-bg);
    color: var(--searchbar-fg);
    outline: none;
}

@media print {
    #morsels-search {
        display: none;
    }
}

#morsels-mdbook-target {
    list-style: none;
}
</style>
"#;

fn get_css_el(base_url: &str, ctx: &PreprocessorContext) -> String {
    let mut output = String::new();

    let add_css = if let Some(TomlBoolean(no_css)) = ctx.config.get("output.morsels.no-css") {
        !(*no_css)
    } else {
        true
    };

    if add_css {
        output.push_str(&format!(
            "<link rel=\"stylesheet\" href=\"{}morsels_assets/search-ui.css\">\n",
            base_url
        ));
        output.push_str(STYLES);
    }

    output
}

fn get_script_els(mode: Option<&Value>, base_url: &str) -> String {
    let mode = if let Some(TomlString(mode)) = mode {
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

    let morsels_js = include_bytes!("morsels.js");
    format!(
"\n
<script src=\"{}morsels_assets/search-ui.bundle.js\" type=\"text/javascript\" charset=\"utf-8\"></script>
<script src=\"{}morsels_assets/mark.min.js\" type=\"text/javascript\" charset=\"utf-8\"></script>\n
<script>
const base_url = '{}';
const mode = {};
{}
</script>",
        base_url, base_url, base_url, mode, std::str::from_utf8(morsels_js).unwrap(),
    )
}

impl Preprocessor for Morsels {
    fn name(&self) -> &str {
        "morsels"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        if let Some(nop_cfg) = ctx.config.get_preprocessor("morsels") {
            if nop_cfg.contains_key("blow-up") {
                anyhow::bail!("Boom!!1!");
            }
        }

        let site_url = if let Some(TomlString(site_url)) = ctx.config.get("output.html.site-url") {
            site_url
        } else {
            "/"
        };

        let init_morsels_el = get_script_els(ctx.config.get("output.morsels.mode"), site_url);

        let mut total_len: u64 = 0;

        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(ch) = item {
                total_len += ch.content.len() as u64;
                ch.content = get_css_el(&site_url, &ctx) + INPUT_EL + &ch.content + &init_morsels_el;
            }
        });

        setup_config_file(&ctx, total_len);

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}
