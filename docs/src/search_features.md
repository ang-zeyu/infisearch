# Search Features

The morsels_search crate provides its own query parser supporting several methods.

## Boolean Operators, Parentheses

`AND` and `NOT` operators are supported, and are used like such in a standard manner.
`OR` operators are not supported, but are implicitly left to the tokenizer (see below for an example).
Parentheses `(...)` can be used to group expressions together.

```
// e.g.
lorem ipsum                 - documents containing either lorem or ipsum.
lorem AND ipsum             - documents with both "lorem" and "ipsum"
lorem AND NOT ipsum         - documents with "lorem" but not "ipsum"
lorem AND NOT (ipsum dolor) - documents with "lorem" but not "ipsum" OR "dolor"
```

## Phrase Queries

Phrase queries are also supported. However, these will only work if the `withPositions` index feature is turned on.

```
// e.g.
"lorem ipsum" - documents containing "lorem" and "ipsum" appearing one after the other
```

## Field Search

Field queries are supported via the following syntax `field_name:`, following the same syntax rules as the `NOT` operator.

```
// e.g.
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

## Behind the scenes Features

This section briefly details some background features not exposed to the user:
- Disjunctive expressions are ranked using the same BM25 model used in lucene
- Pure free-text queries (e.g. "lorem ipsum") use the [WAND algorithm](https://www.elastic.co/blog/faster-retrieval-of-top-hits-in-elasticsearch-with-block-max-wand) to improve query speed
- Background query term proximity ranking: BM25 results are also scaled in an inverse logarithmic manner according to how close disjunctive search expressions are to one another.
