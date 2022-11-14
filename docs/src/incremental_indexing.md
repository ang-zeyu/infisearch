# Incremental Indexing

*Incremental* indexing is also supported by the indexer cli tool.

Detecting **deleted, changed, or added files** is done by storing an **internal file path --> last modified timestamp** map.

To use it, simply pass the `--incremental` or `-i` option when running the indexer.

> You will most likely not need to dabble with incremental indexing, unless your collection is extremely large (e.g. > 200MB).

## Content Based Hashing

The default change detection currently relies on the last modified time in file metadata. This may not always be guaranteed by the tools that generate the files InfiSearch indexes, or be an accurate reflection of whether a file's contents were updated.

If file metadata is *unavailable* for any given file, the file would always be re-indexed as well.

You may specify the `--incremental-content-hash` option in such a case to opt into using a crc32 hash comparison for all files instead. This option should also be specified when running a full index and intending to run incremental indexing somewhere down the line.

It should only be marginally more expensive for the majority of cases, and may be the default option in the future.

## Circumstances that Trigger a Full (Re)Index

Note also, that the following circumstances will forcibly trigger a **full** reindex:
- If the output folder path does not contain any files indexed by InfiSearch
- It contains files indexed by a different version of InfiSearch
- The configuration file (`infi_search.json`) was changed in any way
- Usage of the `--incremental-content-hash` option changed

## Caveats

There are some additional caveats to note when using this option. Whenever possible, try to run a full reindex of the documents, utilising incremental indexing only when indexing speed is of concern -- for example, supporting an "incremental" build mode in static site generators.

### Small Increase in File Size

As one of the core ideas of InfiSearch is to split up the index into many tiny parts, the incremental indexing feature works by "patching" only relevant index files containing terms seen during the current run. Deleted documents are handled using an invalidation bit vector. Hence, there might be a small increase in file size due to these unpruned files.

However, if these "irrelevant" files become relevant again in a future index run, they will be pruned.

### Collection Statistics

Collection statistics used to rank documents will tend to drift off when deleting documents (which also entails updating documents). This is because such documents may contain **terms that were not encountered** during the current run of incremental indexing (from added / updated documents). Detecting such terms is difficult, as there is no guarantee the deleted documents are available anymore. The alternative would be to store such information in a non-inverted index, but that again takes up extra space =(.

As such, the information for these terms may not be "patched". You *may* notice some slight drifting in the relative ranking of documents returned after some number of incremental indexing runs, until said terms are encountered again in some other document.

### File Bloat

When deleting documents or updating documents, old field stores are not removed. This may lead to file bloat after many incremental indexing runs.
