# Tradeoffs

> This page goes *into **excruciating** detail* on how the various indexer presets (`small`, `medium`, ...) are configured for better understanding (if you need to know how to fine-tune the settings), and is an entirely optional read.

## Overview

The possible tradeoffs are marked with ✔️. Those that are likely impossible are marked ❌ (or in other words, you likely need a search server / SaaS for these options). Options that are possible but are relatively undesirable (for which better equivalent options exist) are marked ⚪. The default tradeoff is marked ⭐. Some roughly equivalent / adjacent options are marked ✔️ as it would depend on your collection, use case and some other factors elaborated below.

Latency is labelled in terms of `RTT` (round trip time), the maximum of which is `3`. Also note that the labelled `RTT` times are **maximums**. (e.g. if files are served from cache instead)

| Factor                                                                            | `RTT=0`         | `RTT=1`      | `RTT=2`     | `RTT=3`   |
| -----------                                                                       | -----------     | -----------  | ----------- | --------- |
| Fair Scalability,<br><span style="color: green">Little</span> File bloat          | ⭐ | ⚪ | ⚪ | ⚪
| Fair Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat      | ⚪ | ⚪ | ⚪ | ⚪
| Fair Scalability,<br><span style="color: red">Heavy</span> File bloat             | ⚪ | ⚪ | ⚪ | ⚪
| Good Scalability,<br><span style="color: green">Little</span> File bloat          | ❌ | ✔️ | ✔️ | ⚪
| Good Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat      | ❌ | ✔️ | ✔️ | ⚪
| Good Scalability,<br><span style="color: red">Heavy</span> File bloat             | ❌ | ✔️ | ✔️ | ⚪
| Excellent Scalability,<br><span style="color: green">Little</span> File bloat     | ❌ | ❌ | ❌ | ✔️
| Excellent Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat | ❌ | ❌ | ❌ | ✔️ 
| Excellent Scalability,<br><span style="color: red">Heavy</span> File bloat        | ❌ | ❌ | ✔️ | ⚪
| Beyond Excellent Scalability<br>(consider running a<br>search server / SaaS)      | ❌ | ❌ | ❌ | ❌

> The last column, `RTT=3`, is included only for hypothetical discussion but not present in any of the presets!

### Monolithic Index

Of particular note, the only possible option under `RTT=0` is equivalent to using some other existing client side search library and generating a monolithic prebuilt index **and** document/field store (used for generating result previews).


## Configuration

> This section assumes some knowledge on the options listed below in chapters 4 (barring 4.1) and 5, which will be discussed in briefer detail here:

- Search Configuration: 
  - ⭐⭐ The method of [result preview generation](search_configuration_advanced.md#default-rendering-output--purpose)
  - [`cacheAllFieldStores`](search_configuration.md#search-functionality-options)
- Indexing Configuration:
  - what [fields](./indexer/fields.md) are stored (`do_store`)
  - [`field_store_block_size`](./indexer/fields.md)
  - [`pl_limit`](./indexer/indexing.md#indexing-and-search-scaling-advanced)
  - [`pl_cache_threshold`](./indexer/indexing.md#indexing-and-search-scaling-advanced)

The following sections discusses some combinations of options that generate the tradeoff results in the table above.

### 1. `RTT=0`, Fair Scalability, Little File Bloat

To achieve this result, you will need to ensure **everything** that is potentially needed is retrieved up front.

1. Set `pl_limit` to an arbitrarily large number. This compresses the inverted index into just one or a few files.
1. Ensure `pl_cache_threshold` is set to a very low number (or, at least smaller than the generated inverted index file size), so that all postings lists are loaded up front and cached in memory.
1. You would also want to set `field_store_block_size` to a fairly high number, and correspondingly set `cacheAllFieldStores` to `true`. This allows morsels to load the few field stores during initilisation and persistently cache them.

> ⭐ This is what's being used by this documentation, since it is fairly small.<br><br>Nevertheless, `RTT=1/2` are still very acceptable settings under good network conditions. `RTT=3` may be slightly slow (`~600ms` assuming decent network conditions), but still quite acceptable depending on your use case since it reduces file bloat.<br><br>

### 2. `RTT=1/2`, Good Scalability, Moderate / Heavy File Bloat

The impacts of the two options here are discussed under the 2 main methods of result preview generation in the [earlier chapter](search_configuration_advanced.md).

#### 2.1. Generating Result Previews from Source Files

(`RTT=2`, Little-Moderate file bloat, Good scalability)

Generating result previews from source files greatly reduces file bloat, but it does mean that an extra round (`RTT`) of network requests has to be made to retrieve said source files.

However, it is also more feasible with this option to reduce a round of network requests by **caching all field stores** up front, as field stores here only store the [relative file path / link](indexer/fields.md#reserved-fields) from which to retrieve the source files, and are therefore fairly small.

> For example, assuming each link takes an average of `25` bytes to encode (including json fluff), and `3MB` (ungzipped) is your comfort zone, you can store up to `120000` document links in a single, cached field store!

The relevant options here are `field_store_block_size` and `cacheAllFieldStores` (simply configure them similar to the earlier `RTT=0` case).

#### 2.2. Generating Result Previews from Field Stores

(`RTT=2`, Moderate-Heavy file bloat, Good scalability)

Generating result previews directly from field stores (using the [`do_store`](./indexer/fields.md) option) avoids having to make an extra round of network requests to retrieve said source files.

This however requires fragmenting the field stores, increasing file bloat.

You may want to use this option over **2.1** nevertheless if:
- Result previews cannot be generated from source files (`csv` files)
- You want to increase result preview generation performance (mentioned [here](search_configuration.md#2-from-field-stores))


> Refer [here](https://morsels-search.com) for a demo of what `RTT=2` is like.

#### Improving 2.1 or 2.2 to `RTT=1`

For moderately sized collections, we may also surmise that the **size of the index** (a low-level, compressed inverted index) is often far smaller than the **size of field stores** (which contain the raw document texts).

The idea here therefore is to additionally **cache the index** (using `pl_limit`, `pl_cache_threshold`), removing an entire round of network requests.

This corresponds to the `medium` presets in the previous page.

### 3. Excellent Scalability

The presets here follow from the section directly above, disregarding the assumptions. That is,

#### 3.1. Generating Result Previews from Source Files

(`RTT=3`, Little-Moderate File Bloat, Excellent Scalability)

Per **section 2.1**, The `RTT` compromise is accepted as is in this case, without performing the caching of field stores mentioned.

This is because as the collection grows, we cannot guarantee that document links are at a size that can be feasibly and monolithically cached, although, this is highly unlikely even for the most extreme intended use cases of Morsels (see the earlier example calculation of `120000` documents).

#### 3.2. Generating Result Previews from Field Stores

(`RTT=2`, Heavy File Bloat, Excellent Scalability)

This follows **section 2.2**. No changes are needed here, as both the field stores and index are fragmented.

As the collection size grows however, many fragmented files will inevitably be generated, which may be of concern for your use case.


## Other Performance Considerations

This section discusses some other numbers regarding scaling, and is a *really* optional read.

### Gzip

Gzip can work hand in hand with the morsels' own compression schemes.

With a monolithic index, expect about 3-4x compression ratios **without positions** indexed.

Compression ratios with positions tends to be poorer from some empircal testing, likely since the number of unique positions is fairly large.

When fragmenting the index heavily, gzip would also serve little to no purpose as compression ratios for smaller files tends to be poorer.

> ⭐ Morsels' binary output files are named `.json` by default, so you shouldn't have to manually configure the relevant settings in your server.

### Limits of Scalability

Scaling the tool requires splitting the index into many chunks. Some of these chunks may however exceed the index fragment size limit (`pl_limit`), especially when the chunk contains a very common term (e.g. a stop word like "the"). Splitting such chunks further would be pointless as all such chunks would still have to be retrieved when the term is searched.

This impacts (depending on the configuration):
- The total size of index chunks retrieved for caching during initialisation, that exceed the defined `pl_cache_threshold`.
- The total size of the index chunks that need to be retrieved for a particular query, which is relevant when `pl_cache_threshold` is fairly high (such that no files are cached).

<details>
<summary>Some Data & Estimations</summary>

As a rough estimate from testing, this library should be able to handle text collections < `800MB` with positional indexing and stop words kept.

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

If disabling caching via setting a very high `pl_cache_threshold`, [removing stop words](./indexer/language.md#note-on-stop-words) when indexing would have little to no effect as such terms are already separated into different postings lists and never retrieved unless necessary.

On the other hand, removing stop words with a lower `pl_cache_threshold` would help to avoid caching the "outliers" on the right of the distribution up front, if initial network usage is a concern.

```
# Counts with stop words removed
[7234  209   65   11    5    1    0    0    0    0    0    0    0    0]
```

</details>
