extern crate mdbook;

use std::fs::File;
use std::io::{self, Read};
use std::io::Write;
use std::process::Command;

use anyhow::Error;
use clap::Arg;
use clap::SubCommand;
use clap::App;
use include_dir::{include_dir, Dir};
use mdbook::preprocess::CmdPreprocessor;
use mdbook::book::Book;
use mdbook::book::BookItem;
use mdbook::preprocess::Preprocessor;
use mdbook::preprocess::PreprocessorContext;
use mdbook::renderer::RenderContext;

const SEARCH_UI_DIST: Dir = include_dir!("../search-ui/dist");

pub fn make_app() -> App<'static, 'static> {
    App::new("morsels")
        .about("Morsels preprocessor + renderer for mdbook")
        .subcommand(
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

        Command::new("morsels")
            .current_dir(html_renderer_path)
            .args(&["./", "./morsels_output", "--dynamic"])
            .output()
            .expect("failed to execute indexer process");
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
    style=\"width: 100%; border-radius: 5px; font-size: 22px; border: 1px solid #d2e9ff; outline: none; padding: 0.3em;\"
/>\n\n";

static SCRIPT_EL: &str = r#"<script src="search-ui.bundle.js" type="text/javascript" charset="utf-8"></script>"#;
static CSS_EL: &str = r#"<link rel="stylesheet" href="search-ui.css">"#;

fn get_initialise_script_el() -> &'static str {
    "\n\n<script>
    initMorsels({
        searcherOptions: {
          url: 'morsels_output',
        },
        sourceFilesUrl: '',
        render: {
            enablePortal: true,
        }
    });
</script>"
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

        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(ch) = item {
                ch.content = SCRIPT_EL.to_owned() + CSS_EL + INPUT_EL + &ch.content + get_initialise_script_el();
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}
