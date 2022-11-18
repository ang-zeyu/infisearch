# Search Syntax

InfiSearch provides a few advanced search operators that can be used in the search API. These are also made known to the user using the help icon on the bottom right of the search UI.

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

The [`withPositions`](./indexer/misc.md#indexing-positions) index feature needs to be enabled for this to work (by default it is).

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
