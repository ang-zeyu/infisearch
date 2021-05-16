mod docinfo;
mod fieldinfo;
mod spimireader;
mod spimiwriter;
mod tokenize;
mod utils;
mod worker;

use std::fs;
use std::time::Instant;
use std::env;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::path::Path;
use std::path::PathBuf;

use csv::Reader;
use rustc_hash::FxHashMap;
use walkdir::WalkDir;

use fieldinfo::FieldInfo;
use fieldinfo::FieldInfos;
use worker::Worker;
use worker::MainToWorkerMessage;
use worker::WorkerToMainMessage;
use worker::miner::WorkerMiner;

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
#[macro_use]
extern crate lazy_static;

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

static NUM_THREADS: u32 = 10;
static NUM_DOCS: u32 = 1000;

fn main() {
    let args: Vec<String> = env::args().collect();

    let (input_folder_path, output_folder_path) = resolve_folder_paths(Path::new(&args[1]), Path::new(&args[2]));

    println!("Resolved Paths: {} {}", input_folder_path.to_str().unwrap(), output_folder_path.to_str().unwrap());

    // Initialise ds...
    let mut doc_id_counter = 0;
    let block_number = |doc_id_counter| {
        ((doc_id_counter as f64) / (NUM_DOCS as f64)).ceil() as u32
    };
    let mut spimi_counter = 0;

    let mut field_infos: FieldInfos = FxHashMap::default();
    field_infos.insert("title".to_owned(),       FieldInfo { id: 0, do_store: true, weight: 0.2 });
    field_infos.insert("heading".to_owned(),     FieldInfo { id: 1, do_store: true, weight: 0.3 });
    field_infos.insert("body".to_owned(),        FieldInfo { id: 2, do_store: true, weight: 0.5 });
    field_infos.insert("headingLink".to_owned(), FieldInfo { id: 3, do_store: true, weight: 0.0 });
    field_infos.insert("link".to_owned(),        FieldInfo { id: 4, do_store: true, weight: 0.0 });
    let field_infos_arc: Arc<FieldInfos> = Arc::new(field_infos);
    
    fieldinfo::dump_field_infos(&field_infos_arc, &output_folder_path);

    // Spawn some worker threads!
    let mut workers: Vec<Worker> = Vec::with_capacity(NUM_THREADS as usize);
    let (tx_worker, rx_main) : (Sender<WorkerToMainMessage>, Receiver<WorkerToMainMessage>) = std::sync::mpsc::channel();
    for i in 0..NUM_THREADS {
        let (tx_main, rx_worker) : (Sender<MainToWorkerMessage>, Receiver<MainToWorkerMessage>) = std::sync::mpsc::channel();
        let tx_worker_clone = tx_worker.clone();
        let field_info_clone = Arc::clone(&field_infos_arc);

        workers.push(Worker {
            id: i as usize,
            join_handle: std::thread::spawn(move || worker::worker(i as usize, tx_worker_clone, rx_worker, field_info_clone)),
            tx: tx_main
        });
    }
    Worker::make_all_workers_available(&workers);
    
    let now = Instant::now();
    /* spimireader::merge_blocks(9544, 10, &field_infos_arc, &workers, &rx_main, &output_folder_path);

    print_time_elapsed(now, "Just merge, ");
    Worker::terminate_all_workers(workers);
    return; */

    let field_store_folder_path = output_folder_path.join("field_store");
    if field_store_folder_path.exists() {
        fs::remove_dir_all(&field_store_folder_path).unwrap();
    }
    fs::create_dir(&field_store_folder_path).unwrap();

    for entry in WalkDir::new(input_folder_path) {
        match entry {
            Ok(dir_entry) => {
                if dir_entry.file_type().is_file() && dir_entry.path().extension().unwrap() == "csv" {
                    println!("Reading {}", dir_entry.path().display());

                    let mut rdr = Reader::from_path(dir_entry.path()).unwrap();
                    
                    for result in rdr.records() {
                        let record = result.expect("Failed to unwrap csv record result!");
                        let w = Worker::get_available_worker(&workers, &rx_main);

                        
                        w.send_work(doc_id_counter,
                            vec![("title".to_owned(), record[1].to_string()), ("body".to_owned(), record[2].to_string())],
                            field_store_folder_path.join(format!("{}.json", doc_id_counter)));

                        doc_id_counter += 1;
                        spimi_counter += 1;

                        if spimi_counter == NUM_DOCS {
                            spimiwriter::write_block(NUM_THREADS, &mut spimi_counter, block_number(doc_id_counter), &workers, &rx_main, &output_folder_path);
                        }
                    }
                }
            },
            Err(e) => {
                eprintln!("Error processing entry. {}", e)
            }
        }
    }

    if spimi_counter != 0 && spimi_counter != NUM_DOCS {
        println!("Writing last spimi block");
        spimiwriter::write_block(NUM_THREADS, &mut spimi_counter, block_number(doc_id_counter), &workers, &rx_main, &output_folder_path);
    }

    // Wait on all workers
    Worker::wait_on_all_workers(&workers, &rx_main, NUM_THREADS);
    println!("Number of docs: {}", doc_id_counter);
    print_time_elapsed(now, "Block indexing done!");

    // Merge spimi blocks
    // Go through all blocks at once
    spimireader::merge_blocks(doc_id_counter, block_number(doc_id_counter), &field_infos_arc, &workers, &rx_main, &output_folder_path);

    print_time_elapsed(now, "Blocks merged!");
    Worker::terminate_all_workers(workers); 
}

fn print_time_elapsed(instant: Instant, extra_message: &str) {
    let elapsed = instant.elapsed().as_secs();
    println!("{} {} mins {} seconds elapsed.", extra_message, elapsed / 60, elapsed % 60);
}
