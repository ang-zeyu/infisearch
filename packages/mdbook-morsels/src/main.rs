extern crate mdbook;

use std::fs::File;
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
        for file in SEARCH_UI_DIST.files() {
            let mut output_file = File::create(html_renderer_path.join(file.path())).unwrap();
            output_file.write_all(file.contents()).expect("Failed to copy search-ui assets!");
        }

        let mut command = Command::new("morsels");
        command.current_dir(html_renderer_path).args(&["./", "./morsels_output", "--dynamic"]);

        if let Some(morsels_config_file_path_toml) = ctx.config.get("output.morsels.config") {
            if let TomlString(morsels_config_file_path) = morsels_config_file_path_toml {
                command.arg("-c");
                command.arg(morsels_config_file_path);
            }
        }

        command.output().expect("failed to execute indexer process");
    } else {
        let morsels_preprocessor = Morsels::new();

        if let Some(sub_args) = matches.subcommand_matches("supports") {
            let renderer = sub_args.value_of("renderer").expect("Required argument");

            if renderer == "html" {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        } else {
            let (ctx, book) = CmdPreprocessor::parse_input(&*buf).expect("Preprocess JSON parsing failed");
            let processed_book = morsels_preprocessor.run(&ctx, book).expect("Preprocess processing failed");
            serde_json::to_writer(io::stdout(), &processed_book).unwrap();
            std::process::exit(0);
        }
    }
}

// Preprocessor for adding input search box
pub struct Morsels;

impl Morsels {
    pub fn new() -> Morsels {
        Morsels
    }
}

static INPUT_EL: &str = "\n<input
    type=\"text\"
    id=\"morsels-search\"
    placeholder=\"Search\"
    style=\"width: 100%; border-radius: 5px; font-size: 16px; padding: 0.5em 0.75em; border: 1px solid var(--searchbar-border-color); background: var(--searchbar-bg); color: var(--searchbar-fg); outline: none;\"
/>\n\n";

static SCRIPT_EL: &str = r#"<script src="search-ui.bundle.js" type="text/javascript" charset="utf-8"></script>"#;
static CSS_EL: &str = r#"<link rel="stylesheet" href="search-ui.css">

<style>
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
        "nop-preprocessor"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        /* if let Some(nop_cfg) = ctx.config.get_preprocessor(self.name()) {
            if nop_cfg.contains_key("blow-up") {
                anyhow::bail!("Boom!!1!");
            }
        } */

        let css_el = if let Some(morsels_config_file_path_toml) = ctx.config.get("output.morsels.no-css") {
            if let TomlBoolean(no_css) = morsels_config_file_path_toml {
                if *no_css {
                    ""
                } else {
                    CSS_EL
                }
            } else {
                CSS_EL
            }
        } else {
            CSS_EL
        };

        let site_url = if let Some(html_site_url) = ctx.config.get("output.html.site-url") {
            if let TomlString(site_url) = html_site_url {
                site_url
            } else {
                "/"
            }
        } else {
            "/"
        };

        let init_morsels_el = get_initialise_script_el(ctx.config.get("output.morsels.portal"), site_url);

        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(ch) = item {
                ch.content = SCRIPT_EL.to_owned() + css_el + INPUT_EL + &ch.content + &init_morsels_el;
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}
