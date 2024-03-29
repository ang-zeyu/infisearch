use std::env;
use std::io;
use std::panic;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use infisearch::SOURCE_CONFIG_FILE;
use infisearch::indexer::Indexer;
use infisearch::indexer::input_config::InfiConfig;
use infisearch::assets;
use infisearch::i_debug;

use log::LevelFilter;
use log::{info, error};
use log4rs::Config;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::Appender;
use log4rs::config::Logger;
use log4rs::config::Root;
use path_absolutize::Absolutize;
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(StructOpt, Debug)]
#[structopt()]
struct CliArgs {
    #[structopt(parse(from_os_str))]
    source_folder_path: PathBuf,
    #[structopt(parse(from_os_str))]
    output_folder_path: PathBuf,
    #[structopt(
        short,
        long,
        help = "Preserves the output directory contents, overwriting them as necessary if running a full reindex"
    )]
    preserve_output_folder: bool,
    #[structopt(short, long, parse(from_os_str))]
    config_file_path: Option<PathBuf>,
    #[structopt(long, help = "Allows you to input your indexer configuration via stdin in json format. The entire json should be serialized in one line. Intended for programmatic use.")]
    config_stdin: bool,
    #[structopt(
        short,
        long,
        help = "Prefer incremental indexing if the resources in output folder are available and compatible"
    )]
    incremental: bool,
    #[structopt(
        long,
        help = "Prefer incremental indexing using content hashes. This flag is required even when running a full (re)index, if intending to use incremental indexing runs later"
    )]
    incremental_content_hash: bool,
    #[structopt(long, hidden = true)]
    perf: bool,
    #[structopt(
        long,
        help = "Logging level for the CLI output. Valid options are \"debug\", \"info\", \"warn\", \"error\"",
        default_value = "info"
    )]
    log_level: String,
}

fn get_relative_or_absolute_path(from_path: &Path, path: &Path) -> PathBuf {
    if path.is_relative() {
        from_path.join(path).absolutize().unwrap().to_path_buf()
    } else {
        path.absolutize().unwrap().to_path_buf()
    }
}

fn resolve_folder_paths(
    source_folder_path: &Path,
    output_folder_path: &Path,
    config_file_path: Option<&PathBuf>,
) -> (PathBuf, PathBuf, PathBuf) {
    let cwd_result = env::current_dir();

    match cwd_result {
        Ok(cwd) => {
            let source_return = get_relative_or_absolute_path(&cwd, source_folder_path);

            let output_return = get_relative_or_absolute_path(&cwd, output_folder_path);
            std::fs::create_dir_all(&output_return).expect("Failed to create output directory!");

            let config_return = if let Some(config_raw_file_path) = config_file_path {
                get_relative_or_absolute_path(&cwd, config_raw_file_path)
            } else {
                source_return.join(SOURCE_CONFIG_FILE)
            };
            let config_return = config_return.absolutize().unwrap().to_path_buf();

            (source_return, output_return, config_return)
        }
        Err(e) => {
            panic!("Could not access current directory! {}", e);
        }
    }
}

fn initialize_logger(log_level: &str) {
    let log_level = match log_level {
      "debug" => LevelFilter::Debug,
      "info" => LevelFilter::Info,
      "warn" => LevelFilter::Warn,
      "error" => LevelFilter::Error,
      _ => panic!("Invalid --log-level option specified."),
    };

    let log_config = Config::builder()
        .appender(Appender::builder().build("infisearch_stdout", Box::new(ConsoleAppender::builder().build())))
        .logger(Logger::builder().build("infisearch", log_level))
        .build(Root::builder().appender("infisearch_stdout").build(LevelFilter::Off))
        .unwrap();
    log4rs::init_config(log_config).expect("log4rs initialisation should not fail");
}

fn initialise_config(config_file_path: PathBuf, args: &CliArgs) -> Option<InfiConfig> {
    let config: InfiConfig = if config_file_path.exists() && config_file_path.is_file() {
        InfiConfig::new(std::fs::read_to_string(&config_file_path).unwrap())
    } else if args.config_file_path.is_some() {
        error!("Specified configuration file {} not found!", config_file_path.to_str().unwrap());
        return None;
    } else {
        InfiConfig::default()
    };
    Some(config)
}

fn main() {
    let args: CliArgs = CliArgs::from_args();

    let (input_folder_path, output_folder_path, config_file_path) = resolve_folder_paths(
        &args.source_folder_path,
        &args.output_folder_path,
        args.config_file_path.as_ref(),
    );

    initialize_logger(&args.log_level);

    i_debug!(
        "Resolved Paths:\n  Input folder: {}\n  Output folder: {}\n  Config file: {}",
        input_folder_path.to_str().unwrap(),
        output_folder_path.to_str().unwrap(),
        config_file_path.to_str().unwrap(),
    );

    let config = if args.config_stdin {
        let mut buf = Vec::new();
        for line in io::stdin().lines() {
            if let Ok(line) = line {
                buf.push(line);
            } else {
                panic!("Failed to read config from stdin!");
            }
        }
        InfiConfig::new(buf.join("\n"))
    } else {
        match initialise_config(config_file_path, &args) {
            Some(value) => value,
            None => return,
        }
    };

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        error!("Thread panicked");
        process::exit(1);
    }));

    let mut indexer = Indexer::new(
        &input_folder_path,
        &output_folder_path,
        config,
        args.incremental,
        args.incremental_content_hash,
        args.preserve_output_folder,
        args.perf,
    );

    info!("Finding files to index.");

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
                error!("Error processing entry. {}", e)
            }
        }
    }

    info!("All documents indexed, merging results.");

    let total_documents = indexer.finish_writing_docs();

    assets::write_infisearch_assets(&output_folder_path.join("assets"));

    info!("{} documents indexed.", total_documents);
}
