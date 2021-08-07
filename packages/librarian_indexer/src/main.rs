use std::time::Instant;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use path_slash::PathExt;

use librarian_indexer::LibrarianConfig;

use csv::Reader;
use walkdir::WalkDir;

/*
Cargo new <name> build --release/dev(default) check doc --open
cargo install

modules
- pub use     reexports
- mod filename / filename of file within directory having same basename as current file;


Strings:
- string literal != string
- string slice: &str / strvar[1..2]
- single indexing not allowed
- push_str(str) / push(char) / + (takes ownership of first param) / format!("{} {} {}", ...) - no ownership
- .chars() - individual unicodes, .bytes() - individual bytes

Traits:
trait X {
    ... default implementations / definitions
}

impl X for ...

fn foo(...) -> impl X { ... }
fn foo(param: impl X or &impl X)

shorthand for

fn foo<T: X>(param: T or &T)


Multiple:
fn foo(param: (impl X + Y))
fn foo<T: X + Y>(param: T)
fn some_function<T, U>(t: &T, u: &U) -> i32
    where T: Display + Clone, U: Clone + Debug {...}


Errors
- Result can be unwrap() or expect(msg) or unwrap_or_else(closure)
- ? operator
- fn main() -> Result<(), Box<dyn Error>>

Closure
|x| { ... }
- auto inferred, but can be type annotated as well
- captures environment variables (some overhead incurred) -- in incremental, overriding order below, auto inferred
  - FnOnce (all closures) - takes ownership
  - FnMut (closures that move captured variables) - mutably borrows
  - Fn (closures that don't move) - borrows
move |x| {...}
  - forces ownership of environment variables
can be passed around...
functions can be passed around as well: fn(i32) -> i32

Iterator (a trait, implements fn next(&mut self) -> Option<Self::Item>, where type Item (associated type))
- lazily evaluated
- vec![1,2,3].iter()           mutable, but items are immutable references
- vec![1,2,3].into_iter()      mutable, but items are owned
- vec![1,2,3].iter_mut()       mutable, but items are mutable references
- for x in iterator {...}
- v1.iter().sum() .collect()                                                                  consuming adaptor, takes ownership of the iterator
- v1.iter().map(|x| x + 1) .filter() .skip(n) .zip(another it, iterates alternatingly)        iterator adaptors, changes iterators into different iterators

Smart Pointer (e.g. String, Vec<T>)
- in contrast to & pointers that only borrow data, these often own the data
- usually implemented with structs
  - Deref / DerefMut trait - allows to behave like a reference, * can be used (made into *(x.deref()) to avoid ownership): fn deref(&self) -> &T
    - deref coercion (e.g. &String -> &str, Box<String> -> &String): mut -> immut, immut -> immut, mut -> mut
  - Drop trait - code run when out of scope): fn drop(&mut self) { ... }. Call manually with drop(...)
- Heap value allocation (Box<T>)
  - why: type whose size can't be known at compile time, large amount of data to transfer ownership but ensure no copying, proxy to type implementing a trait
  - Box::new(...)
- Reference counting (Rc<T>): for graphs
  - **single threaded** usage only!
  - Rc::new(...), Rc::clone(&...) - dosen't deepy copy, just increases ref count, Rc::strong_count
  - Reference cycle prevention: Rc::downgrade(...) (no ownership - increases Rc::weak_count) -> Weak<T>. Rc::upgrade to return Option<Rc<T>>
- Interior mutability (immut type exposes API for mutating) RefCell<T>, single ownership / thread
  - works via **unsafe** code (runtime enforcement of borrowing - panics if violated)
  - .borrow() -> Ref<T> (multiple) / .borrow_mut() -> RefMut<T> (one) ....
  - Combine with Rc to have multiple owners of mutable
- Reference Cycles

Concurrency:
- x = thread::spawn, x.join().unwrap()
- message passing: 
  - channel: transmitter - receiver, closed if either is dropped
    - (tx, rx) = mpsc::channel(); (multiple producer, single consumer)
      - tx.send(val).unwrap()  - takes ownership
      - tx.clone() for multiple producer
      - rx.recv().unwrap() (blocking), rx.try_recv().unwrap() (if there is one)
      - for val in rx {...} (ends when channel closes)
- std::sync::Mutex<T>
  - m = Mutex::new(val...); v = m.lock().unwrap(); *v = ...;
    - lock - acquire lock, unwrap - have the thread panic if unable to acquire lock. Combined, returns MutexGuard pointer
    - lock auto dropped out of scope
    - Combine with Arc::new, Arc::clone to allow multiple threads to hold it, or impl Send trait (variant of Rc)
- Sync trait - safe to be reference from multiple threads
- Send trait - transferable between threads

Patterns
- _
- a | b
- let { x: a, y: b } = somestruct...
- let (a,b) = (1,2)
- while / if let
- match {
    Point { x, .. } => 
    (a, ..)
    Some(_)
    1..=10 =>
    'a'..'j'
}
*/


fn resolve_folder_paths(source_folder_path: &Path, output_folder_path: &Path) -> (PathBuf, PathBuf) {
    let cwd_result = env::current_dir();

    match cwd_result {
        Ok(cwd) => {
            let source_return = if source_folder_path.is_relative() {
                let mut cwd = cwd.clone();
                cwd.push(source_folder_path);
                cwd
            } else {
                PathBuf::from(source_folder_path)
            };
        
            let mut output_return = cwd;
            if output_folder_path.is_relative() {
                output_return.push(output_folder_path);
            } else {
                output_return = PathBuf::from(output_folder_path);
            }

            (source_return, output_return)
        },
        Err(e) => {
            panic!("Could not access current directory! {}", e);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let (input_folder_path, output_folder_path) = resolve_folder_paths(Path::new(&args[1]), Path::new(&args[2]));

    println!("Resolved Paths: {} {}", input_folder_path.to_str().unwrap(), output_folder_path.to_str().unwrap());

    let config_file_path = input_folder_path.join(PathBuf::from("_librarian_config.json"));
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
