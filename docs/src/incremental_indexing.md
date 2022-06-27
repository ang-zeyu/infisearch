# Incremental Indexing

*Incremental* indexing is also supported by the indexer cli tool.

Detecting **deleted, changed, or added files** is done by storing an **internal file path --> last modified timestamp** map.

To use it, simply pass the `--incremental` or `-i` option when running the indexer.

> Again, you will most likely not need to dabble with incremental indexing, unless your collection is extremely large (e.g. > 200MB).

## How it Works

As the core idea of Morsels is to split up the index into many tiny parts, the incremental indexing feature works by "patching" only the files which were updated during the current run. This means that at search time, the same amount of index files are retrieved and searched through as before, to reduce the number of network requests.

This is in contrast to a more traditional "segment" based approach you might find in search servers, whereby each incremental indexing run generates an entirely separate "segment", and segments are merged together at runtime (during search). While this makes sense for traditional search tools, it may unfortunately generate too many network requests for index files and search overhead from merging files, something Morsels is trying to minimise.

## Content Based Hashing

The default change detection currently relies on the last modified time in file metadata. This may not always be guaranteed by the tools that generate the files Morsels indexes, or be an accurate reflection of whether a file's contents were updated.

If file metadata is *unavailable* for any given file, the file would always be re-indexed as well.

You may specify the `--incremental-content-hash` option in such a case to opt into using a crc32 hash comparison for all files instead. This option should also be specified when running a full index and intending to run incremental indexing somewhere down the line.

It should only be marginally more expensive for the majority of cases, and may be the default option in the future.

## Circumstances that Trigger a Full (Re)Index

Note also, that the following circumstances will forcibly trigger a **full** reindex:
- If the output folder path does not contain any files indexed by morsels
- It contains files indexed by a different version of morsels
- The configuration file (`morsels_config.json`) was changed in any way
- Usage of the `--incremental-content-hash` option changed

## Caveats

There are some additional caveats to note when using this option. Whenever possible, try to run a full reindex of the documents, utilising incremental indexing only when indexing speed is of concern -- for example, updating the index repeatedly when developing this documentation (although, the mdbook plugin this documentation is built on currently dosen't do that).

### Collection Statistics

Collection statistics used to rank documents will tend to drift off when deleting documents (which also entails updating documents). This is because such documents may contain **terms that were not encountered** during the current run of incremental indexing (from added / updated documents). Detecting such terms is difficult, as there is no guarantee the deleted documents are available anymore. The alternative would be to store such information in a non-inverted index, but that again takes up extra space =(.

As such, the information for these terms may not be "patched". You *may* notice some slight drifting in the relative ranking of documents returned after some number of incremental indexing runs, until said terms are encountered again in some other document.

### File Bloat

When deleting documents or updating documents, old field stores are not removed. This may lead to file bloat after many incremental indexing runs.
