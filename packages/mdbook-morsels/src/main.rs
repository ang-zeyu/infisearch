extern crate mdbook;

use std::fs::{self, File};
use std::io::Write;
use std::io::{self, Read};
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

const SEARCH_UI_DIST: Dir = include_dir!("../search-ui/dist");

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

        let assets_output_dir = html_renderer_path.join("morsels_assets");
        fs::create_dir_all(&assets_output_dir).expect("mdbook-morsels: Failed to create assets output directory");
        for file in SEARCH_UI_DIST.files() {
            let mut output_file = File::create((&assets_output_dir).join(file.path())).expect("mdbook-morsels: Failed to open asset write handler");
            output_file.write_all(file.contents()).expect("mdbook-morsels: Failed to copy search-ui assets!");
        }

        let morsels_config_path = if let Some(TomlString(morsels_config_file_path)) = ctx.config.get("output.morsels.config") {
            ctx.root.join(morsels_config_file_path)
        } else {
            ctx.root.join("_morsels_config.json")
        };

        if !morsels_config_path.exists() || !morsels_config_path.is_file() {
            let mut init_config_command = Command::new("morsels");
            init_config_command.current_dir(ctx.root.clone()).args(&["./", "./morsels_output", "--init"]);
            init_config_command.arg("-c");
            init_config_command.arg(&morsels_config_path);
            init_config_command.output().expect("mdbook-morsels: failed to create default configuration file");
        }

        let mut command = Command::new("morsels");
        command.current_dir(html_renderer_path)
            .args(&["./", "./morsels_output", "--dynamic"])
            .arg("-c")
            .arg(morsels_config_path);

        command.output().expect("mdbook-morsels: failed to execute indexer process");
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

// Preprocessor for adding input search box
pub struct Morsels;

static INPUT_EL: &str = "\n<input
    type=\"text\"
    id=\"morsels-search\"
    placeholder=\"Search\"
    style=\"width: 100%; border-radius: 5px; font-size: 16px; padding: 0.5em 0.75em; border: 1px solid var(--searchbar-border-color); background: var(--searchbar-bg); color: var(--searchbar-fg); outline: none;\"
/>\n\n";

static STYLES: &str = r#"<style>
.morsels-root {
    --morsels-border: 3px solid var(--table-header-bg);
    --morsels-fg: var(--fg);
    --morsels-bg: var(--bg);
    --morsels-item-border: 1px solid var(--table-border-color);
    --morsels-item-sub-border:  1px solid var(--table-border-color);
    --morsels-dropdown-input-separator-bg: var(--table-header-bg);
    --morsels-title-bg: var(--table-header-bg);
    --morsels-title-hover-fg: var(--bg);
    --morsels-title-hover-bg: var(--fg);
    --morsels-heading-bg: var(--table-alternate-bg);
    --morsels-heading-hover-bg: var(--table-header-bg);
    --morsels-body-hover-bg: var(--table-alternate-bg);
    --morsels-highlight: var(--search-mark-bg);
    --morsels-fine-print-fg: var(--fg);
    --morsels-loading-bg: var(--fg);
    --morsels-scrollbar-bg: var(--sidebar-bg);
    --morsels-scrollbar-thumb-bg: var(--sidebar-non-existant);
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
</style>"#;

fn get_assets_els(base_url: &str, ctx: &PreprocessorContext) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        r#"<script src="{}morsels_assets/search-ui.bundle.js" type="text/javascript" charset="utf-8"></script>"#,
        base_url
    ));

    let add_css = if let Some(TomlBoolean(no_css)) = ctx.config.get("output.morsels.no-css") {
        if *no_css {
            false
        } else {
            true
        }
    } else {
        true
    };

    if add_css {
        output.push_str(&format!(
            r#"<link rel="stylesheet" href="{}morsels_assets/search-ui.css">"#,
            base_url
        ));
        output.push_str(STYLES);
    }

    output
}

fn get_initialise_script_el(enable_portal: Option<&Value>, base_url: &str) -> String {
    let enable_portal = if let Some(enable_portal) = enable_portal {
        if let TomlString(_str) = enable_portal {
            "'auto'"
        } else if let TomlBoolean(do_enable) = enable_portal {
            if *do_enable {
                "true"
            } else {
                " false "
            }
        } else {
            "'auto'"
        }
    } else {
        "'auto'"
    };

    format!(
        "\n\n<script>
    initMorsels({{
        searcherOptions: {{
          url: '{}morsels_output',
        }},
        sourceFilesUrl: '',
        render: {{
            enablePortal: {}
        }}
    }});
</script>",
        base_url, enable_portal
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

        let init_morsels_el = get_initialise_script_el(ctx.config.get("output.morsels.portal"), site_url);

        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(ch) = item {
                ch.content = get_assets_els(&site_url, &ctx) + INPUT_EL + &ch.content + &init_morsels_el;
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}
