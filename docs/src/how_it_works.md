# How it Works

The core idea of this tool is to split up a monolithic postings list into many smaller files (hence *"Morsels"*), organised by the indexed terms. Multiple index files are batched into the same file, keeping to `16383` bytes (configurable) as much as possible.

On the client, only supporting information (e.g. dictionary, document lengths, field infos) is retrieved on startup, which is usually less than a few MB even for fairly large collections (`< 1gb`).

The index files of searched terms will be requested only on-demand from a static file server.

## Limits

The practicality / scalability of this tool is bound by 2 factors:

### Size of the largest index chunk

While the index is split into many chunks, some chunks may exceed the "split size" of `16383` bytes at times. This occurs when the chunk contains a very common term (e.g. a stop word like "the"). While we could further split the information for this term into multiple chunks, all such chunks will still have to be retrieved when the term is searched, diminishing the benefit.

Certain [indexing options](./indexing_configuration.md) like removing positions and pre-caching larger chunks on startup are available to alleviate this to some extent, though not infinitely.

#### Estimations

The test collection used during development is a pure-text `380mb` .csv file, with positional indexing enabled. No stop word removal is done.

Under these settings, the largest chunk weighed `5mb`.

As an estimate, this library should be able to handle collections < `800mb` with positional indexing. Without it, the index shrinks 3-4 fold, making it potentially possible to index collections `~2gb` in size.


### Hardware capabilities

Device capabilities is also another concern (e.g. performance when ranking and populating results), although in practice, you should be hitting limits due to the first factor long before experiencing issues with this.


## Other Design Choices

### WebWorker built in

Most of the search library operates on a WebWorker where it matters (e.g. setup), so you don't have to worry about blocking the UI thread.

Document field store population is however done on the main thread, as copying large documents to-and-fro WebWorker interfaces incurs substantial overhead.

Search UI (@morsels/search-ui) related functionalities, for example SERP generation, is also done on the main thread.
One of the main reasons is that there is simply no way of parsing html (the original html document can be used as a field store) faster than the implementations provided by the browser.

### Wasm / Rust for the searcher

The search portion of the project was developed in typescript for a very large part. While usable, switching to a wasm / rust implementation yielded 2-3 fold performance benefits on average, and never slower.

The usual wasm overheads of transferring large, complex data structures across the boundary don't quite apply for the use cases here either, as only index chunks are transferred over in raw byte representation.

### Rust for the indexer

Rust was chosen for the indexer mainly as this was my first project in Rust. =P

In retrospect, performance is critical for indexing fairly large collections nonetheless, making Rust a fairly good choice for the indexer.

A javascript implementation was also trialed in early stages of development (see the commit history). While javascript has come a long way in performance, it is inevitably still leaps behind a compiled language.
