# Search Features

This page is mostly *informational*, and explains some search features in more detail:

## Ranking Model

Query expressions are ranked using the BM25 model. A soft disjunctive maximum of a document's field's scores is then calculated. By default, titles, `<h1>` headings, other headings, and the rest of the text are indexed into 4 separate fields.

**Query term proximity ranking** is Morsels' highlight here, and is enabled by default. Results are scaled according to how close search expressions are to one another, greatly improving search relevance.

## Advanced Search Syntax

Morsels provides a few advanced search features that are made known to the user using the help icon on the bottom right of the search UI.

### Boolean Operators, Parentheses

`AND` and `NOT` and inversion operators are supported.
`OR` is the default behaviour; Documents are ranked according to the BM25 model.
Parentheses `(...)` can be used to group expressions together.

```
weather +sunny  - documents that may contain "weather" but must contain "sunny"
weather -sunny  - documents containing "weather" and do not have "sunny"
~cloudy         - all documents that do not contain "gloomy"
~(ipsum dolor)  - all documents that do not contain "ipsum" and "dolor"
```

### Phrase Queries

Phrase queries are also supported by enclosing the relevant terms in `"..."`.

```
"sunny weather" - documents containing "sunny weather"
```

The [`withPositions`](./indexer/indexing.md#miscellaneous-options) index feature needs to be enabled for this to work (by default it is).

### Field Search

Field queries are supported via the following syntax `field_name:`:

```
title:sunny              - documents containing "sunny" in the title
heading:(+sunny +cloudy) - documents with both "lorem" and "ipsum" in headings only
body:gloomy              - documents with "gloomy" elsewhere
```

### Wildcard Search

You can also perform suffix searches on any term using the `*` character:

```
run* - searches for "run", "running"
```

In most instances, an [*automatic*](./search_configuration.md#automatic-suffix-search) wildcard suffix search is also performed on the last query term that the user is still typing.

### Escaping Search Operators

All search operators can also be escaped using `\`:

```
\+sunny
\-sunny
\(sunny cloudy\)
\"cloudy weather\"
"phrase query with qu\"otes"
title\:lorem
```

## Low-Level Inverted Index Format

Some efficient, high-return compression schemes are also employed, so you get all these features without much penalty.
- Gap encoding for document ids, positions
- Byte-and-bit-wise variable integer encoding

To facilitate decompression efficiency of such a low-level format, most of the search library is powered by WebAssembly (Rust) as such.

This documentation for example, which has all features enabled, generates a main index file of just 23KB, and a dictionary of 10KB.

## WebWorker Built-in

Most of the search library also operates on a WebWorker, so you can deliver the best UX without blocking the UI thread.

Retrieval of stored document fields (the raw document text for generating result previews and highlighting) and result preview generation is however done on the main thread, as copying many large documents to-and-fro WebWorker interfaces incurs substantial overhead.

## Persistent Caching

Persistent caching is achieved through use of the [Cache](https://developer.mozilla.org/en-US/docs/Web/API/Cache) API, which backs service workers and has excellent support in modern browsers.
