# Search API

You can also interface with InfiSearch through its API.

## Setup

Under the global `infisearch` variable, you can instantiate an instance of the `Searcher` class.

```ts
const searcher = new infisearch.Searcher({
    url: 'https://... the index output directory ...'
});
```

The constructor parameter uses the same options as `infisearch.init`, refer to this [page](./search_configuration.md#search-functionality-options) for the other available options.

**Initialising States**

Setup is also async and and mostly proceeds in the WebWorker. You can  use the `setupPromise` and `isSetupDone` interfaces to optionally show UI initialising states.

```ts
searcher.setupPromise.then(() => {
    assert(searcher.isSetupDone, true);
});
```

**Retrieving Enum Values**

If you have an [enum field](./indexer/fields.md#field-storage-storagetext), you can retrieve all its possible values like such:

```ts
const enumValues: string[] = await searcher.getEnumValues('weather');

> console.log(enumValues)
['sunny', 'rainy', 'warm', 'cloudy']
```

## Querying

Next, you can create a `Query` object, which obtains and ranks the result set. 

```ts
const query: Query = await searcher.runQuery('sunny weather');
```

The `Query` object follows this interface.

```ts
interface Query {
    /**
     * Original query string.
     */
    public readonly query: string,
    /**
     * Total number of results.
     */
    public readonly resultsTotal: number,
    /**
     * Returns the next top N results.
     */
    public readonly getNextN: (n: number) => Promise<Result[]>,
    /**
     * Freeing a query manually is required since its results live in the WebWorker.
     */
    public readonly free: () => void,
}
```

### Filtering Enum Values

Filter document results with [enum fields](./indexer/fields.md#field-storage-storagetext) by passing an additional parameter.

```ts
const query: Query = await searcher.runQuery('weather', {
  enumFilters: {
    weather: [
      null,    // Use null to include documents that have no enum values
      'sunny',
      'warm',
    ]
  }
});
```

## Loading Document Texts

Running a query alone probably isn't very useful. You can get a `Result` object using the `getNextN` function.

```ts
const results: Result[] = await query.getNextN(10);
```

A `Result` object stores the fields of the indexed document.

```ts
const fields = results[0].fields;

> console.log(fields)
{
  texts: [
    ['_relative_fp', 'relative_file_path/of_the_file/from_the_folder/you_indexed'],
    ['title', 'README'],
    ['h1', 'README'],
    ['headingLink', 'description'],
    ['heading', 'Description'],
    ['body', 'InfiSearch is a client-side search solution made for static sites, .....'],
    // ... more headingLink, heading, body fields ...
  ],
  enums: {
    weather: 'cloudy',
    reporter: null,
  }
}
```

- `texts`: fields are stored as an array of `[fieldName, fieldText]` pairs in the order they were seen.

   This ordered model is more complex than a regular key-value store, but enables the detailed content hierarchy you see in InfiSearch' UI: *Title > Heading > Text under heading*
- `enums`: This stores the enum values of the document. Documents missing specific enum values will be assigned `null`.

## Memory Management

As InfiSearch uses a WebWorker to run things, you would also need to perform some memory management.

Once you are done with a `Query` (e.g. if a new query was run), call `free()` on the `query` object.

```ts
query.free();
```

Search interfaces usually live for the entire lifetime of the application. If you need to do so however, you should also free the `Searcher` instance:

```ts
searcher.free();
```

## Convenience Methods

A `Result` object also exposes 2 other convenience functions that may be useful to help deal with the positional format of the `text` type field stores.

### 1. Retrieving Singular Fields as KV Stores

Certain fields will only occur once in every document (e.g. titles, `<h1>` tags). To retrieve these easily, you can use the `getKVFields` method:

```ts
const kvFields = result.getKVFields('link', '_relative_fp', 'title', 'h1');

> console.log(kvFields)
{
  "_relative_fp": "...",
  "title": "..."
  // Missing fields will not be populated
}
```

Only the first `[fieldName, fieldText]` pair for each field will be populated into the `fields` object.

**Tip: Constructing a Document Link**

If you haven't manually added any links to your source documents, you can use the `_relative_fp` field to construct one, by concatenating it to a base URL for example. Any links added via the [`data-morsels-link`](./linking_to_others.md) attribute are also available under the `link` field.

### 2. Highlighting and Linking 'Heading' and 'Body' Excerpts

To establish the relationship between adjacent *heading*, *body* and *headingLink* fields in particular, you can call the `getHeadingBodyExcerpts` method.

```ts
const bodyHeadingMatchResults: Segment[] = result.getHeadingBodyExcerpts();
```

This returns an array of *Segments*. Each *Segment* represents a chunk of `fieldText`.

**Sorting and Choosing Segments**

```ts
interface Segment {
  /**
   * 'heading' types come from 'heading' fields,
   * while 'body' and 'heading-body' types come from 'body' fields.
   */
  type: 'heading' | 'body' | 'heading-body',

  /**
   * This will only be present if type === 'heading-body',
   * and points to another Segment with type === 'heading'.
   */
  heading?: Segment,

  /**
   * This will only be present if type === 'heading' | 'heading-body',
   * and points to the heading's id, if any.
   */
  headingLink?: string,

  /**
   * How many terms were matched in this segment.
   */
  numTerms: number,
}
```

You would select and display a **few best segments** only. To rank them, you could for example first priortise segments with a greater `numTerms` matched, then tie-break by the `type` of the segment. This is up to your UI!

**Text Highlighting**

```ts
interface Segment {
  ...

  highlight: (addEllipses: boolean = true) => (string | HTMLElement)[],
  highlightHTML: (addEllipses: boolean = true) => string,

  window: { pos: number, len: number }[],   // Character position and length
  text: string,                             // original string
}

```

You can perform text highlighting manually using the original `text` and the closest `window` of term matches, or automatically using the `highlight()` and `highlightHTML()` methods.

The `highlight()` method wraps term matches in a `<mark>` element, truncates surrounding text, and adds leading and trailing ellipses elements. An example output is as follows:

```ts
[
  <span class="morsels-ellipses"> ... </span>,
  ' ... text before ... ',
  <mark class="morsels-highlight">highlighted</mark>,
  ' ... text after ... ',
  <span class="morsels-ellipses"> ... </span>,
]
```

To interact with the `(string | HTMLElement)[]` output safely (strings are unescaped) and efficiently, you could use the `.append(...segment.highlight())` [DOM API](https://developer.mozilla.org/en-US/docs/Web/API/Element/append) with the spread operator.

You can also call `highlightHTML()` which returns a single escaped HTML string. This is less efficient, but easier to use with `.innerHTML = '...'`.
