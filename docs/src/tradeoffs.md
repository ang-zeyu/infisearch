# Tradeoffs

> This page goes *into **excruciating** detail* on how the various indexer presets (`small`, `medium`, ...) are configured for better understanding (if you need to know how to fine-tune the settings), and is an entirely optional read.

## Overview

The possible tradeoffs are marked ✔️. Those likely impossible are marked ❌ (in other words, you likely need a search server / SaaS). Options that are possible but have better equivalent options are marked ⚪. The default tradeoff is marked ⭐. Some roughly equivalent / adjacent options are marked ✔️ as it would depend on your collection, use case and some other factors elaborated below.

Latency is labelled in terms of `RTT` (round trip time), the maximum of which is `3`. Also note that the labelled `RTT` times are **maximums**. (e.g. if files are served from cache instead)

| Factor                                                                            | `RTT=0`         | `RTT=1`      | `RTT=2`
| -----------                                                                       | -----------     | -----------  | -----------
| Ok Scaling,<br><span style="color: green">Little</span> File bloat            | ⭐ | ⚪ | ⚪
| Ok Scaling,<br><span style="color: #ff8a0f">Moderate</span> File bloat        | ⚪ | ⚪ | ⚪
| Ok Scaling,<br><span style="color: red">Heavy</span> File bloat               | ⚪ | ⚪ | ⚪
| Good Scaling,<br><span style="color: green">Little</span> File bloat          | ❌ | ❌ | ❌
| Good Scaling,<br><span style="color: #ff8a0f">Moderate</span> File bloat      | ❌ | ✔️ | ⚪
| Good Scaling,<br><span style="color: red">Heavy</span> File bloat             | ❌ | ⚪ | ⚪
| Excellent Scaling,<br><span style="color: green">Little</span> File bloat     | ❌ | ❌ | ❌
| Excellent Scaling,<br><span style="color: #ff8a0f">Moderate</span> File bloat | ❌ | ❌ | ❌
| Excellent Scaling,<br><span style="color: red">Heavy</span> File bloat        | ❌ | ❌ | ✔️
| Beyond Excellent Scaling<br>(consider running a<br>search server / SaaS)      | ❌ | ❌ | ❌

**Monolithic Index**

Of particular note, the only possible option under `RTT=0` is equivalent to using some other existing client side search library and generating a monolithic prebuilt index **and** document/field store (used for generating result previews).


## Configuration

The following sections discusses some combinations of options that generate the tradeoff results in the table above.

### 1. `RTT=0`, Ok Scaling, Little File Bloat

To achieve this result, you will need to ensure **everything** that is potentially needed is retrieved up front.

1. Set `pl_limit` to an arbitrarily large number. This compresses the inverted index into just one or a few files.
1. Ensure `pl_cache_threshold` is set to a very low number (or, at least smaller than the generated inverted index file size), so that all postings lists are loaded up front and cached in memory.
1. You would also want to set `num_docs_per_store` to a fairly high number to generate few field stores, and correspondingly set `cacheAllFieldStores` to `true` which persistently caches them.

> ⭐ This is what's being used by this documentation, since it is fairly small.<br><br>Nevertheless, `RTT=1/2` are still very acceptable settings under good network conditions.

### 2. `RTT=1`, Good Scaling, Moderate File Bloat

For moderately sized collections, we may also surmise that the **size of the index** (a low-level, compressed inverted index) is often far smaller than the **size of field stores** (which contain the raw document texts).

The idea here therefore is to additionally **cache the index** (using `pl_limit`, `pl_cache_threshold`), removing an entire round of network requests. This however requires fragmenting the field stores, increasing file bloat.

This corresponds to the `medium` presets in the previous page.

### 3. `RTT=2`, Even Better Scaling, Heavy File Bloat

At even larger collection sizes, the index may not be monolithically retrievable, and must be sharded.

This splits the index into many fragments of configurable size on disk using `pl_limit`. Only the necessary fragments are retrieved based on the query.


## Other Performance Considerations

This section discusses some other numbers regarding scaling, and is a *really* optional read.

### Gzip

Gzip can work hand in hand with the morsels' own compression schemes.

With a monolithic index, expect about 3-4x compression ratios **without positions** indexed.

Compression ratios with positions tends to be poorer from some empircal testing, likely since the number of unique positions is fairly large. As such, the generated index files are use a custom file format of `.mls`.

When fragmenting the index heavily, gzip also serves little to no purpose as compression ratios for smaller files tends to be poorer.

### Scaling Limits

Scaling the tool requires splitting the index into many chunks. Some of these chunks may however exceed the index fragment size limit (`pl_limit`), especially when the chunk contains a very common term (e.g. a stop word like "the"). Splitting such chunks further would be pointless as all such chunks would still have to be retrieved when the term is searched.

This impacts:
- The total size of index chunks retrieved for caching during **initialisation**, that exceed the defined `pl_cache_threshold`.
- The total size of the index chunks that need to be retrieved **for a particular query**, which is relevant when `pl_cache_threshold` is fairly high (such that no files are cached).

As a rough estimate from testing, this library should be able to handle text collections &lt; `800MB` with positional indexing and stop words kept. Some data and estimations are available [here](https://github.com/ang-zeyu/morsels/blob/main/docs/src/numbers.md).
