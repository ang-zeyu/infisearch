# Tradeoffs

When configuring morsels, there are several tradeoffs you need to keep in mind, which varies greatly on depending on your collection and use case. 

This chapter outlines the possible tradeoffs you can make and summarises the relevant options you would want to keep in mind.

## Possible Tradeoffs

The possible tradeoffs you can make are marked with ✔️. Those that are likely impossible are marked ❌, or in other words, you need a search server / SaaS for these options. Some options that are possible but are relatively undesirable (for which better equivalent options exist) are marked 😩.

Latency is labelled in terms of `RTT` (round trip time), the maximum of which is `3`.
Scalability and file bloat are inevitably labelled more **subjectively**.

Also note that the labelled `RTT` times are **maximums**. (namely, if files are served from cache instead)

| Factor                                                                            | `RTT=0`         | `RTT=1`      | `RTT=2`     | `RTT=3`   |
| -----------                                                                       | -----------     | -----------  | ----------- | --------- |
| Fair Scalability,<br><span style="color: green">Little</span> File bloat          | ✔️ | 😩 | 😩 | 😩
| Fair Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat      | 😩 | 😩 | 😩 | 😩
| Fair Scalability,<br><span style="color: red">Heavy</span> File bloat             | 😩 | 😩 | 😩 | 😩
| Good Scalability,<br><span style="color: green">Little</span> File bloat          | ❌ | ❌ | ✔️ | 😩
| Good Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat      | ❌ | ✔️ | 😩 | 😩
| Good Scalability,<br><span style="color: red">Heavy</span> File bloat             | ❌ | ✔️ | 😩 | 😩
| Excellent Scalability,<br><span style="color: green">Little</span> File bloat     | ❌ | ❌ | ❌ | ✔️
| Excellent Scalability,<br><span style="color: #ff8a0f">Moderate</span> File bloat | ❌ | ❌ | ❌ | ✔️ 
| Excellent Scalability,<br><span style="color: red">Heavy</span> File bloat        | ❌ | ❌ | ✔️ | 😩
| Beyond Excellent Scalability<br>(consider running a<br>search server / SaaS)      | ❌ | ❌ | ❌ | ❌

> Some roughly equivalent / nearby options are still marked ✔️ (vs 😩), since the labels are subjective.

Of particular note, the only possible option under `RTT=0` is equivalent to using some other existing client side search library and generating a monolithic prebuilt index.

You may still want to use morsels since it packages a search UI, or, if you prefer the simplicity of a cli indexer tool (e.g. for CI).


## Configuration

> This section assumes you have read chapters 4 (barring 4.1) and 5, the options will be discussed in briefer detail here.

The relevant options you will want to keep in mind are:
- Search Configuration: 
  - ⭐⭐ The method of [result preview generation](search_configuration.md#default-rendering-output--purpose)
  - [`cacheAllFieldStores`](search_configuration.md#search-library-options)
- Indexing Configuration:
  - what fields are stored (`do_store`)
  - [`field_store_block_size`](indexing_configuration.md#fields_config)
  - [`pl_limit`](indexing_configuration.md#indexing_config)
  - [`pl_cache_threshold`](indexing_configuration.md#indexing_config)
  - [`num_stores_per_dir`](indexing_configuration.md#indexing_config)

The following sections discusses some combinations of options that generate the outputs in the table above.

### 1. `RTT=0`, Fair Scalability, Little File Bloat

To achieve this result, you will need to ensure **everything** that is **potentially** needed is retrieved up front.

- Set `pl_limit` to an arbitrarily large number. This compresses the inverted index into one or a few files.
- Ensure `pl_cache_threshold` is set to a very low number (or at least smaller than the inverted index file size), so that the postings list are loaded up front and cached in memory.
- You would also want to set `field_store_block_size` to a fairly high number, and correspondingly set `cacheAllFieldStores` to `true`. This allows morsels to load the few field stores during initilisation and persistently cache them.


### 2. `RTT=1/2`, Good Scalability, Moderate / Heavy File Bloat

The tradeoffs here a a little more complex; The impacts of various options are discussed under the 2 main methods of result preview generation.

#### 2.1. Generating Result Previews from Source Files

While generating result previews from source files greatly reduces file bloat, it does mean that an extra round (`RTT`) of network requests has to be made to retrieve said source files.

Therefore, the tradeoff here is between **file bloat** and **`RTT`**.

However, it is also more feasible with this option to remove a round of network requests by **compressing and caching** all field stores up front.
This is because in this option, field stores only store the relative file path from which to retrieve the source files, and are therefore fairly small.

For example, assuming each link takes an average of `25` bytes to encode (including json fluff), and `3MB` (ungzipped) is your "comfort zone", you can store up to `120000` document links in a single, cached field store!

The relevant options are `pl_cache_threshold` and `field_store_block_size` (configure similar to the earlier `RTT=0` case).

> ⭐ This is the default settings! (`RTT=2`, Little file bloat, Good scalability)


#### 2.2. Generating Result Previews from Field Stores

(`RTT=1`, Moderate-Heavy file bloat, Good scalability)

It is also possible to achieve another trade off by using this method of preview generation.

As mentioned, generating result previews directly from field stores (making sure to specify `do_store` on the appropriate fields) avoids the extra mentioned round of network requests to retrieve said source files.

Moreover, for moderately sized collections, we may surmise that the **size of the index** is often far smaller than the **size of field stores**.

The idea here therefore is to **cache the index** (using `pl_limit`, `pl_cache_threshold`) and fragment the **field stores** (`field_store_block_size`), therefore reducing another `RTT`.

### 3. Excellent Scalability

The settings here follow from the section directly above, disregarding the compromises. That is,

#### 3.1. Generating Result Previews from Source Files

(`RTT=3`, Excellent Scalability, Little-Moderate File Bloat)

The `RTT` compromise is accepted as is, without performing the caching mentioned in section 2.1.

This is because as the collection grows, we cannot guarantee that document links are at a size that can be feasibly and monolithically cached.

#### 3.2. Generating Result Previews from Field Stores

(`RTT=2`, Excellent Scalability, Heavy File Bloat)

Per section 2.2, one simply needs to avoid the assumption that the index can be cached.

Scalability is then ensured here by fragmenting both the index (using `pl_limit`) and field stores (`field_store_block_size`).
