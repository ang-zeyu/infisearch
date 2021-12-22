# Search Features

This page outlines the available search features common to both `@morsels/search-ui` and `@morsels/search-lib`.

## Boolean Operators, Parentheses

`AND` and `NOT` operators are supported.
`OR` operators are not supported, but is implicitly left to the tokenizer (see below for an example).
Parentheses `(...)` can be used to group expressions together.

```
lorem ipsum                 - documents containing either lorem or ipsum.
lorem AND ipsum             - documents with both "lorem" and "ipsum"
lorem AND NOT ipsum         - documents with "lorem" but not "ipsum"
lorem AND NOT (ipsum dolor) - documents with "lorem" but not ("ipsum" OR "dolor")
```

## Phrase Queries

Phrase queries are also supported. However, these will not work if the [`withPositions`](./indexer/indexing.md#miscellaneous-options) index feature is disabled.

```
"lorem ipsum" - documents containing "lorem" and "ipsum" appearing one after the other
```

## Field Search

Field queries are supported via the following syntax `field_name:`, following the same syntax rules as the `NOT` operator.

```
title:lorem             - documents containing "lorem" in the field "title"
title:(lorem AND ipsum) - documents with both "lorem" and "ipsum" in the
                          field "title" only
lorem AND title:ipsum   - documents with "ipsum" in the title and "lorem" in any field
```

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

## Other Details

This section briefly details some non user-facing features.

### WebWorker Built-in

Most of the search library operates on a WebWorker where it matters (e.g. setup), so you don't have to worry about blocking the UI thread.

Population of stored document fields is however done on the main thread, as copying large documents to-and-fro WebWorker interfaces incurs substantial overhead.

Search UI (@morsels/search-ui) related functionalities, for example SERP generation, is also done on the main thread.
One of the main reasons is that there is simply no way of parsing html (the original html document can be used as an alternative to storing document fields) faster than the implementations provided by the browser.

### Ranking Specifics

Top-level disjunctive expressions (e.g. `lorem ipsum`) are ranked using the BM25 model.

Pure free-text queries (e.g. "lorem ipsum") also use the [WAND algorithm](https://www.elastic.co/blog/faster-retrieval-of-top-hits-in-elasticsearch-with-block-max-wand) to improve query speed, although, the benefits should be marginal for most cases.

Query term proximity ranking is also supported and enabled by default - results are scaled in an inverse logarithmic manner according to how close disjunctive search expressions are to one another.
