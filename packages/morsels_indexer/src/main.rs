use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use morsels_indexer::i_debug;
use morsels_indexer::MorselsConfig;

use log::LevelFilter;
use log::error;
use log4rs::Config;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::Appender;
use log4rs::config::Logger;
use log4rs::config::Root;
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(StructOpt, Debug)]
#[structopt(name = "morsels")]
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
    #[structopt(long, help = "Initialises the configuration file in the source folder. Does not run any indexing.")]
    config_init: bool,
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
}

fn get_relative_or_absolute_path(from_path: &Path, path: &Path) -> PathBuf {
    if path.is_relative() {
        from_path.join(path)
    } else {
        PathBuf::from(path)
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
                source_return.join("morsels_config.json")
            };

            (source_return, output_return, config_return)
        }
        Err(e) => {
            panic!("Could not access current directory! {}", e);
        }
    }
}

fn initialize_logger() {
    let log_config = Config::builder()
        .appender(Appender::builder().build("morsels_stdout", Box::new(ConsoleAppender::builder().build())))
        .logger(Logger::builder().build("morsels_indexer", LevelFilter::Info))
        .build(Root::builder().appender("morsels_stdout").build(LevelFilter::Off))
        .unwrap();
    log4rs::init_config(log_config).expect("log4rs initialisation should not fail");
}

fn main() {
    let args: CliArgs = CliArgs::from_args();

    let (input_folder_path, output_folder_path, config_file_path) = resolve_folder_paths(
        &args.source_folder_path,
        &args.output_folder_path,
        args.config_file_path.as_ref(),
    );

    initialize_logger();

    i_debug!(
        "Resolved Paths:\n{}\n{}\n{}",
        input_folder_path.to_str().unwrap(),
        output_folder_path.to_str().unwrap(),
        config_file_path.to_str().unwrap(),
    );

    if args.config_init {
        morsels_indexer::Indexer::write_morsels_source_config(MorselsConfig::default(), &config_file_path);
        return;
    }

    let config: MorselsConfig = if config_file_path.exists() && config_file_path.is_file() {
        MorselsConfig::new(std::fs::read_to_string(&config_file_path).unwrap())
    } else if args.config_file_path.is_some() {
        error!("Specified configuration file {} not found!", config_file_path.to_str().unwrap());
        return;
    } else {
        MorselsConfig::default()
    };

    let mut indexer = morsels_indexer::Indexer::new(
        &output_folder_path,
        config,
        args.incremental,
        args.incremental_content_hash,
        args.preserve_output_folder,
    );

    let now = if args.perf { Some(Instant::now()) } else { None };

    let input_folder_path_clone = input_folder_path.clone();

    for entry in WalkDir::new(input_folder_path) {
        match entry {
            Ok(dir_entry) => {
                if !dir_entry.file_type().is_file() {
                    continue;
                }

                let path = dir_entry.path();
                let relative_path = path.strip_prefix(&input_folder_path_clone).unwrap();

                indexer.index_file(&input_folder_path_clone, path, relative_path);
            }
            Err(e) => {
                error!("Error processing entry. {}", e)
            }
        }
    }

    indexer.finish_writing_docs(now);
}
