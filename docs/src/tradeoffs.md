# Tradeoffs

When configuring morsels, there are several tradeoffs you need to keep in mind, which varies greatly on depending on your collection and use case. 

This chapter outlines the possible tradeoffs you can make and summarises the relevant options you would want to keep in mind.

## Possible Tradeoffs

The possible tradeoffs you can make are marked with ✔️. Those that are likely impossible are marked ❌, or in other words, you likely need a search server / SaaS for these options. Some options that are possible but are relatively undesirable (for which better equivalent options exist) are marked 😩. The default tradeoff is marked ⭐.

Latency is labelled in terms of `RTT` (round trip time), the maximum of which is `3`. Also note that the labelled `RTT` times are **maximums**. (e.g. if files are served from cache instead)

| Factor                                                                            | `RTT=0`         | `RTT=1`      | `RTT=2`     | `RTT=3`   |
| -----------                                                                       | -----------     | -----------  | ----------- | --------- |
| Fair Scalability,<br><span style="color: green">Little</span> File bloat          | ✔️ | 😩 | 😩 | 😩
| Fair Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat      | 😩 | 😩 | 😩 | 😩
| Fair Scalability,<br><span style="color: red">Heavy</span> File bloat             | 😩 | 😩 | 😩 | 😩
| Good Scalability,<br><span style="color: green">Little</span> File bloat          | ❌ | ⭐ | ✔️ | 😩
| Good Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat      | ❌ | ✔️ | ✔️ | 😩
| Good Scalability,<br><span style="color: red">Heavy</span> File bloat             | ❌ | ✔️ | ✔️ | 😩
| Excellent Scalability,<br><span style="color: green">Little</span> File bloat     | ❌ | ❌ | ❌ | ✔️
| Excellent Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat | ❌ | ❌ | ❌ | ✔️ 
| Excellent Scalability,<br><span style="color: red">Heavy</span> File bloat        | ❌ | ❌ | ✔️ | 😩
| Beyond Excellent Scalability<br>(consider running a<br>search server / SaaS)      | ❌ | ❌ | ❌ | ❌

> Some roughly equivalent / nearby options are marked ✔️ as it would depend on your collection, use case and some other factors elaborated below.

### Monolithic Index

Of particular note, the only possible option under `RTT=0` is equivalent to using some other existing client side search library and generating a monolithic prebuilt index.

You may still want to use morsels since it packages a search UI, or, if you prefer the simplicity of a cli indexer tool (e.g. for CI).


## Configuration

> This section assumes you have read chapters 4 (barring 4.1) and 5, the options will be discussed in briefer detail here.

The relevant options you will want to keep in mind are:
- Search Configuration: 
  - ⭐⭐ The method of [result preview generation](search_configuration.md#default-rendering-output--purpose)
  - [`cacheAllFieldStores`](search_configuration.md#search-library-options)
- Indexing Configuration:
  - what [fields](./indexer/fields.md) are stored (`do_store`)
  - [`field_store_block_size`](./indexer/fields.md)
  - [`pl_limit`](./indexer/indexing.md#search-performance)
  - [`pl_cache_threshold`](./indexer/indexing.md#search-performance)

The following sections discusses some combinations of options that generate the outputs in the table above.

### 1. `RTT=0`, Fair Scalability, Little File Bloat

To achieve this result, you will need to ensure **everything** that is **potentially** needed is retrieved up front.

1. Set `pl_limit` to an arbitrarily large number. This compresses the inverted index into just one or a few files.
1. Ensure `pl_cache_threshold` is set to a very low number (or at least smaller than the inverted index file size), so that all postings lists are loaded up front and cached in memory.
1. You would also want to set `field_store_block_size` to a fairly high number, and correspondingly set `cacheAllFieldStores` to `true`. This allows morsels to load the few field stores during initilisation and persistently cache them.

> ⭐ This is what's being used by this documentation, since it is fairly small.<br><br>Nevertheless, `RTT=1/2` are still very acceptable settings under good network conditions. `RTT=3` may be slightly slow (`~600ms` assuming decent network conditions), but still quite acceptable depending on your use case since it reduces file bloat.<br><br>

### 2. `RTT=1/2`, Good Scalability, Moderate / Heavy File Bloat

The impacts of the two options here are discussed under the 2 main methods of result preview generation discussed [earlier](search_configuration.md#options-for-generating-result-previews).

#### 2.1. Generating Result Previews from Source Files

(`RTT=2`, Little-Moderate file bloat, Good scalability)

Generating result previews from source files greatly reduces file bloat, but it does mean that an extra round (`RTT`) of network requests has to be made to retrieve said source files.

However, it is also more feasible with this option to remove a round of network requests by **caching all field stores** up front, as field stores only store the [relative file path / link](indexer/fields.md#special-fields) from which to retrieve the source files, and are therefore fairly small.

For example, assuming each link takes an average of `25` bytes to encode (including json fluff), and `3MB` (ungzipped) is your "comfort zone", you can store up to `120000` document links in a single, cached field store!

The relevant options here are `field_store_block_size` and `cacheAllFieldStores` (simply configure them similar to the earlier `RTT=0` case).

#### 2.2. Generating Result Previews from Field Stores

(`RTT=2`, Moderate-Heavy file bloat, Good scalability)

Generating result previews directly from field stores (making sure to specify `do_store` on the appropriate fields) avoids the extra mentioned round of network requests to retrieve said source files.

This however requires fragmenting the field stores, increasing file bloat.

You may want to use this option over **2.1** nevertheless if:
- Result previews cannot be generated from source files (`csv` files)
- You want to increase result preview generation performance (as mentioned [here](search_configuration.md#2-from-field-stores))


> Refer to the demo [here](https://ang-zeyu.github.io/morsels-demo-1/) to see what `RTT=2` is like.

#### Improving Either Option to `RTT=1`

For moderately sized collections, we may also surmise that the **size of the index** (a low-level, compressed inverted index) is often far smaller than the **size of field stores** (which contain the raw document texts).

The idea here therefore is to additionally **cache the index** (using `pl_limit`, `pl_cache_threshold`), removing an entire round of network requests.

⭐ This is also the default tradeoff made, using method 2.1.

To further reduce the size of the index to be cached, therefore extending the feasibility of caching the entire index, take a look at the [other options](#other-options) section.

### 3. Excellent Scalability

The settings here follow from the section directly above, disregarding the compromises. That is,

#### 3.1. Generating Result Previews from Source Files

(`RTT=3`, Little-Moderate File Bloat, Excellent Scalability)

Per **section 2.1**, The `RTT` compromise is accepted as is in this case, without performing the caching of field stores mentioned.

This is because as the collection grows, we cannot guarantee that document links are at a size that can be feasibly and monolithically cached, although, this is highly unlikely even for the most extreme cases (see the earlier example calculation of `120000` documents).

#### 3.2. Generating Result Previews from Field Stores

(`RTT=2`, Heavy File Bloat, Excellent Scalability)

Per **section 2.2**. No changes are needed here, as both the field stores and index are fragmented.


### Other Options

There are 2 other options worth highlighting that can help reduce the index size, both of which are not active by default.

- [`ignore_stop_words`](./indexer/language.md#note-on-stop-words)
- [`with_positions`](./indexer/indexing.md#miscellaneous-options)<br>
  Positional information takes up a considerable (up to **3-4** times larger) proportion of the index size!

If you are willing to forgo some features (e.g. phrase queries, boolean queries of stop words) in return for reducing the index size, you can enable / disable these options as appropriate.

This would be especially useful if configuring for a **monolithic index** (`RTT=0`, Fair Scalability, Little File Bloat), or any [other options](#improving-either-option-to-rtt1) which cache the index (not field stores) up front, as it reduces the index size to be retrieved on initialisation.

### Limits of Scalability

Scaling the tool requires splitting the index into many chunks. Some of these chunks may however exceed the default `pl_limit` of `16383` bytes, especially when the chunk contains a very common term (e.g. a stop word like "the"). While the information for this term could be further split into multiple chunks, this would be almost pointless as all such chunks would still have to be retrieved when the term is searched.

Since larger index chunks are cached according to the `pl_cache_threshold`, the limit is relevant mostly during startup / initialisation only. That is, scalability is limited by the **total size** of **index chunks which exceed the `pl_cache_threshold`** that will be **retrieved upfront**.

If configuring for a much higher `pl_cache_threshold` however, such that no files are cached, then the limit is imposed during **search** by the total size of the index chunks that need to be retrieved for the query.

#### Estimations

As a rough estimate from testing, this library should be able to handle **text collections < `800MB` with positional indexing**.

The following distribution of index chunk file sizes **(before gzip)** under the default `pl_limit` was produced with:
- A `380MB` **csv** corpus (no html soup!)
- **Duplicated once** to total about `760MB`, and 19088 documents

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

If disabling caching via setting a very high `pl_cache_threshold`, [removing stop words](./indexer/language.md#note-on-stop-words) when indexing would have little to no effect as such terms are already separated into different postings lists and never retrieved unless necessary.

On the other hand, removing stop words with a lower `pl_cache_threshold` would help to avoid caching the "outliers" on the right of the distribution up front, if initial network usage is a concern.

```
# Counts with stop words removed
[7234  209   65   11    5    1    0    0    0    0    0    0    0    0]
```

