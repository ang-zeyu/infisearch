The following distribution of index chunk file sizes (before gzip) under the default `pl_limit` was produced with:
- A `380MB` **csv** corpus (no HTML soup!)
- Duplicated once to total about `760MB`, and 19088 documents

```
# Counts
[7335  219   76   13    7    14    4     1     1     1     0     0     0     1]
# (Left) Bin Edges, in KB
[0     100   250  500   750  1000  2000  3000  4000  5000  6000  7000  8000  9000]
```

Most of the index chunks are well below the default `pl_cache_threshold` of `1048576` bytes, while the select few above it totals roughly `45MB`. Therefore, on startup, `45MB` of index chunks are fetched and cached. The remaining bulk of index chunks are retrieved on-demand.

##### Disabling Positions

Without positional indexing, the index shrinks **3-4 fold**, making it potentially possible to index collections `~2gb` in size, or even more.

In addition, large postings lists are all but removed in this case:

```
# Counts with positional information removed
[4350    0    0    0    0    0    0    0    0    0    0    0    0    0]
```

##### Removing Stop Words

If disabling caching via setting a very high `pl_cache_threshold`, [removing stop words](./language.md#note-on-stop-words) when indexing would have little to no effect as such terms are already separated into different postings lists and never retrieved unless necessary.

On the other hand, removing stop words with a lower `pl_cache_threshold` would help to avoid caching the "outliers" on the right of the distribution up front, if initial network usage is a concern.

```
# Counts with stop words removed
[7234  209   65   11    5    1    0    0    0    0    0    0    0    0]
```
