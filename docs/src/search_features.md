# Search Features

This page outlines the available search features.

## Boolean Operators, Parentheses

`AND` and `NOT` operators are supported.
`OR` operators are not supported, but its function is implicitly left to the tokenizer (see below for an example).
Parentheses `(...)` can be used to group expressions together.

```
lorem ipsum                 - documents containing either lorem OR ipsum
lorem AND ipsum             - documents with both "lorem" and "ipsum"
lorem AND NOT ipsum         - documents with "lorem" but not "ipsum"
lorem AND NOT (ipsum dolor) - documents with "lorem" but not ("ipsum" OR "dolor")
```

## Phrase Queries

Phrase queries are also supported by enclosing the relevant terms in `"..."`.

```
"lorem ipsum" - documents containing "lorem" and "ipsum" appearing one after the other
```

The [`withPositions`](./indexer/indexing.md#miscellaneous-options) index feature needs to be enabled for this to work (by default it is).

## Field Search

Field queries are supported via the following syntax `field_name:`, following the same syntax rules as the `NOT` operator.

```
title:lorem             - documents containing "lorem" in the field "title"
title:(lorem AND ipsum) - documents with both "lorem" and "ipsum" in the
                          field "title" only
lorem AND title:ipsum   - documents with "ipsum" in the title and "lorem" in any field
```

## Wildcard Search

You can also perform suffix searches on any term using the `*` character:

```
run* - searches for "run", "running"
```

In most instances, an [*automatic*](./search_configuration.md#automatic-suffix-search) wildcard suffix search is also performed on the last query term that the user is still typing.

## Escaping Search Operators

All search operators can also be escaped using the `\` character like such:

```
lorem\ AND ipsum            - interpreted literally as "lorem AND ipsum"
\NOT lorem                  - interpreted literally as "NOT lorem"
\(not a parentheses group\)
\"not a phrase query\"
"phrase query with qu\"ote inside"
title\:lorem
```

## Non User-Facing Features

### WebWorker Built-in

Most of the search library operates on a WebWorker where it matters (e.g. setup, query ranking), so you don't have to worry about blocking the UI thread.

<details>

<summary>Exceptions</summary>

Retrieval of stored document fields (the raw document text for generating result previews and highlighting) is however done on the main thread, as copying many large documents to-and-fro WebWorker interfaces incurs substantial overhead.

Search UI related functionalities, for example result preview generation, is also done on the main thread.
The main rationale is that there is simply no way of parsing HTML faster than implementations provided by the browser. (the original HTML document can be used as an alternative to storing document fields for result preview generation)

</details>

### Low-Level Inverted Index Format

Some efficient, high-return compression schemes are also employed, so you get all these features without much penalty.
- Gap encoding for document ids, positions
- Byte-and-bit-wise variable integer encoding

To facilitate decompression efficiency of such a low-level format, most of the search library is powered by WebAssembly (Rust) as such.

### Ranking Specifics

Most query expressions (e.g. free text queries like `lorem ipsum`) are ranked using the BM25 model, while `AND` and `()` operators sum the respective BM25 scores of their operands. A soft disjunctive maximum of document's field scores is calculated.

**Query term proximity ranking** is also supported and enabled by default for top-level expressions, when the `with_positions` index [feature](./indexer/indexing.html#miscellaneous-options) is enabled. Results are scaled according to how close search expressions are to one another.
