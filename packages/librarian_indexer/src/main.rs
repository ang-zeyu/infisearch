use std::time::Instant;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use path_slash::PathExt;

use librarian_indexer::LibrarianConfig;

use csv::Reader;
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct CliArgs {
    #[structopt(parse(from_os_str))]
    source_folder_path: PathBuf,
    #[structopt(parse(from_os_str))]
    output_folder_path: PathBuf,
    #[structopt(short, long, parse(from_os_str))]
    config_file_path: Option<PathBuf>,
}

fn get_relative_or_absolute_path(from_path: &Path, path: &Path) -> PathBuf {
    if path.is_relative() {
        from_path.join(path).canonicalize().unwrap()
    } else {
        PathBuf::from(path)
    }
}

fn resolve_folder_paths(source_folder_path: &Path, output_folder_path: &Path, config_file_path: Option<&PathBuf>) -> (PathBuf, PathBuf, PathBuf) {
    let cwd_result = env::current_dir();

    match cwd_result {
        Ok(cwd) => {
            let source_return = get_relative_or_absolute_path(&cwd, &source_folder_path);
        
            let output_return = get_relative_or_absolute_path(&cwd, &output_folder_path);

            let config_return = if let Some(config_raw_file_path) = config_file_path {
                get_relative_or_absolute_path(&cwd, &config_raw_file_path)
            } else {
                source_return.join("_librarian_config.json")
            };

            (source_return, output_return, config_return)
        },
        Err(e) => {
            panic!("Could not access current directory! {}", e);
        }
    }
}

fn main() {
    let args: CliArgs = CliArgs::from_args();

    let (input_folder_path, output_folder_path, config_file_path) = resolve_folder_paths(
        &args.source_folder_path,
        &args.output_folder_path,
        args.config_file_path.as_ref(),
    );

    println!("Resolved Paths: {} {} {}",
        input_folder_path.to_str().unwrap(),
        output_folder_path.to_str().unwrap(),
        config_file_path.to_str().unwrap(),
    );

    let config: LibrarianConfig = if config_file_path.exists() && config_file_path.is_file() {
        let config_raw = std::fs::read_to_string(config_file_path).unwrap();
        serde_json::from_str(&config_raw).expect("_librarian_config.json does not match schema!")
    } else {
        LibrarianConfig::default()
    };

    let mut indexer = librarian_indexer::Indexer::new(
        &output_folder_path,
        config,
    );

    let now = Instant::now();

    let input_folder_path_clone = input_folder_path.to_str().unwrap().to_owned();

    for entry in WalkDir::new(input_folder_path) {
        match entry {
            Ok(dir_entry) => {
                if !dir_entry.file_type().is_file() {
                    continue;
                }

                let path = dir_entry.path();
                let extension = path.extension().unwrap();
                if extension == "csv" {
                    let mut rdr = Reader::from_path(path).unwrap();
                    
                    for result in rdr.records() {
                        let record = result.expect("Failed to unwrap csv record result!");

                        indexer.index_document(
                            vec![
                                ("title", record[1].to_string()),
                                ("body", record[2].to_string()),
                                ("link", record[0].to_string()),
                            ]
                        );
                    }
                } else if extension == "html" {
                    indexer.index_html_document(
                        path.strip_prefix(&input_folder_path_clone).unwrap().to_slash().unwrap(),
                        std::fs::read_to_string(path).expect("Failed to read file!")
                    );
                }
            },
            Err(e) => {
                eprintln!("Error processing entry. {}", e)
            }
        }
    }
    
    indexer.finish_writing_docs(Option::from(now));
}
