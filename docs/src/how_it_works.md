# How it Works

The core idea of this tool is to split up a monolithic postings list into many smaller files (hence *"Morsels"*), organised by the indexed terms. Multiple postings lists are batched into the same file as long as the starting point of the last postings list dosen't exceed `65535` bytes.

On the client, only supporting information (e.g. dictionary, document lengths, field infos) is retrieved on startup, which is usually less than a few MB even for fairly large collections (`< 1gb`).

The postings lists of searched terms will be requested only on-demand from a static file server.

## Other Design Choices

### Rust for the indexer

Rust was chosen for the indexer mainly as this was my first project in Rust.

In retrospect, performance is critical for indexing fairly large collections nonetheless, making Rust a good choice for the indexer.

A javascript implementation was also trialed in early stages (see the commit history). While javascript has come a long way in performance, it is inevitably still leaps behind a compiled language.

### Wasm / Rust for the searcher

The search portion of the project was developed in typescript for a very large part. While usable, switching to a wasm / rust implementation yielded 2-3 fold performance benefits on average, and never slower.

### WebWorker built in

Most of the search library operates on a WebWorker, so you don't have to worry about blocking the UI thread.

Document field population is however, is done on the main thread, as copy large documents to-fro WebWorker interfaces is costly (messages) and / or awkward (SharedArrayBuffer) at the moment.
