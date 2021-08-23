# Dynamic Indexing

Dynamic, or "incremental" indexing is also supported by the indexer cli tool.

Detecting deleted, changed, or added files is done by storing an internal file path / last modified metadata map.

To enable it, simply pass the `--dynamic` or `-d` option when running the indexer.

## How it Works

As the core idea of Morsels is to split up the index into many tiny parts (and not more than necessary), the dynamic indexing feature works by "patching" only the files which were updated during the current run. This means that at search time, the same amount of index files are retrieved and searched through as before, to reduce the number of network requests.

This is in contrast to a "segment" based approach whereby each dynamic indexing run generates an entirely separate "segment", and segments are merged together at runtime. While this makes sense for traditional search tools, it may unfortunately generate too many network requests for index files and search overhead from merging files, something Morsels is trying to minimise.

## Caveats

There are certain caveats to note when using this option. Whenever possible, try to run a full reindex of the documents, utilising dynamic indexing only when indexing speed is of concern -- for example, updating the index immediately when writing this documentation!

This should be automatic for most use cases -- if the output folder path does not contain any files indexed by morsels and the `--dynamic` option is specified, this is similar to not specifying the `--dynamic` option at all.

### Collection Statistics

Collection statistics will tend to drift off when deleting documents (which also entails updating documents). This is because such documents may contain terms that were not encountered during the current run of dynamic indexing (from added / updated documents). As such, the files containing the information for these terms would not be "patched". As a result, you *may* notice some slight drifting in the relative ranking of documents returned after some number of dynamic indexing runs.

### File Metadata

The change detection currently relies on the last modified time in file metadata. This may not be available or accurate on all systems, or guaranteed by the tools that generate the files Morsels indexes.

If file metadata is *unavailable* for any given file, the file would always be re-indexed.
