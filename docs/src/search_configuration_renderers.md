# Altering HTML Outputs

<style>
    .alert-warning {
        color: #856404;
        background-color: #fff3cd;
        border-color: #ffeeba;
    }
    .alert {
        position: relative;
        padding: 0.75rem 1.25rem;
        margin-bottom: 1rem;
        border: 1px solid transparent;
        border-radius: 0.25rem;
    }
</style>

>  This page covers some APIs that allow you to customise some small parts of the HTML output structure, if required.

Some use cases for this include:
- The default HTML structure is not sufficient for your styling needs
- You need to attach additional event listeners to elements
- You want to override or insert additional content sourced from custom fields / static content (e.g. a footer)
- You want to change the [default use case](./search_configuration_advanced.md#default-rendering-output--purpose) of following through on a result preview to its source document entirely

> 💡 If you only need to style the dropdown or search popup, you can include your own css file to do so [and / or override the variables](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/styles/search.css) exposed by the default css bundle.

These API options are similarly specified under the `uiOptions` key of the root configuration object.

```ts
initMorsels({
    uiOptions: {
        // ...
    }
});
```

## The `h` function

`h`

Almost all APIs here are passed a `h` function. This is an *optional* helper function you may use to create elements.

The method signature is as such:

```ts
export type CreateElement = (
  // Element name
  name: string,

  // Element attribute map
  attrs: { [attrName: string]: string },

  /*
   Child elements (HTMLElement) OR text nodes (just put the string)
   string parameters utilise .textContent,
   so you don't have to worry about escaping potentially malicious content
  */
  ...children: (string | HTMLElement)[]
) => HTMLElement;
```

## Passing Custom Options

`opts`

All renderer functions are also passed an `opts` parameter. This is the original input object that you provided to the `initMorsels` call, with default parameters populated at this point.

```ts
opts = export interface Options {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
  isMobileDevice: () => boolean,
  otherOptions: any
}
```

If you need to include some custom options (e.g. an API base url), you can use the `otherOptions` key, which is guaranteed to be untouched by morsels.


## Changing Supporting Parts of the UI

The options here are intended for changing small, supporting elements, which can be especially useful for localizing the UI.

#### `headerRender(h, opts, query: Query): HTMLElement`

This API renders the "10 results found" text by default.

This can also be used to render messages like "*Did you mean <u>spelling</u>?*", or any information that you'd like to place as a header.

**`query.resultsTotal`**

This property of the `query` parameter gives the total number of results.

**`query.queryParts`**

This parameter passed to the `headerRender` function is the parsed structure of the input query string.

The structure is fairly detailed, `console.log` it out to see what it looks like, our check out the [source](https://github.com/ang-zeyu/morsels/blob/main/packages/search/lib/parser/queryParser.ts)!

#### `loadingIndicatorRender(h, opts, isSetup: boolean, isInitial: boolean)`

This API renders the loading indicator.

While the search library is doing initialising work, the `isSetup` parameter is set to `true`.

The `isInitial` boolean is `true` when the user runs the first query, where there are no results yet. You may use this parameter to change the look of the indicator in subsequent queries. In the default design, subsequent queries move the spinning indicator to the top right.

#### `fsBlankRender(h, opts): HTMLElement`

This method renders the *Start searching above!* text when the search box is empty in the fullscreen UI. The dropdown UI which is hidden in such a case.

#### `errorRender(h, opts): HTMLElement`

Renders the element when an unexpected error occurs.

## Rendering Search Results

The below 2 *mutually exclusive* sets of APIs render the results for all document matches. The first, simpler set of APIs are "building blocks" of the second. Reconfiguring the second API would also invalidate the first.

### 1. Rendering a Single Result

The APIs under this key are grouped under `uiOptions.resultsRenderOpts` (instead of `uiOptions.XXX`).

These APIs are more suited for performing smaller modifications for the default use case, for example, displaying an additionally indexed field (e.g. an icon).

<div style="height:1px"></div>

`listItemRender(h, opts, fullLink, resultTitle, resultHeadingsAndTexts, fields): HTMLElement`

This API renders the result for a single document match.

| Parameter   | Description |
| ----------- | ----------- |
| `fullLink`                 | full resource link of the document |
| `resultTitle`              | the first extracted `title` field of the document, if any |
| `resultHeadingsAndTexts`   | An array of `string` or `HTMLElement` intended to be used as the last parameter of `h`.<br><br>This contains the highlighted heading-body pair matches, or body-only matches rendered from `headingBodyRender` and `bodyOnlyRender` further below. |
| `fields`                   | All stored fields for the document, as positioned `[fieldName, fieldValue]` pairs. Useful if adding additional fields. |

The following example shows the default implementation, and how to add an additional field, `subtitle`, to each result.

```ts
const subTitleField = fields.find(field => field[0] === 'subtitle');

const linkEl = h(
  'a', { class: 'morsels-link' },
  h('div', { class: 'morsels-title' }, title,
    h('div', { class: 'morsels-subtitle' }, (subTitleField && subTitleField[1]) || '')
  ),
  ...bodies
);

if (fullLink) {
  linkEl.setAttribute('href', fullLink);
}

return h(
  'li', { class: 'morsels-list-item' },
  linkEl,
);
```

#### 1.1 `listItemRender` supporting APIs

The remaining 3 APIs below are building blocks of `listItemRender`, responsible for generating its `resultTitle` and `resultHeadingsAndTexts` parameters.

```ts
interface SearchUiRenderOptions {
  // Renders a "heading" field,
  // along with the highlighted "body" fields that follow it (in document order)
  headingBodyRender?: (
    h: CreateElement,

    // The highlighted elements under .morsels-heading. Intended to be used with the 'h' function.
    headingHighlights: (HTMLElement | string)[],    

    // The highlighted elements under .morsels-body. Intended to be used with the 'h' function.
    bodyHighlights: (HTMLElement | string)[], 

    // Url of the document + The matching heading's id, if any
    href?: string                             
  ) => HTMLElement,


  // Renders highlighted "body" fields without a heading preceding it
  bodyOnlyRender?: (
    h: CreateElement,

    // The highlighted elements under .morsels-body. Intended to be used with the 'h' function.
    bodyHighlights: (HTMLElement | string)[], 
  ) => HTMLElement,


  highlightRender?: (
    h: CreateElement,

    // matched term
    matchedPart: string,                      
  ) => HTMLElement,
}
```

### 2. Rendering All Results

`async resultsRender(h, opts, config, results, query): Promise<HTMLElement[]>`

This API renders the results for *all* document matches.

Some examples use cases are:
- Altering the html output structure substantially (e.g. displaying results in a tabular form)
- Calling external API calls to retrieve additional info for generating result previews.
  - For this reason, this is also the only `async` API

For example, the default implementation does the following:
1. Check the `config.fieldInfos` if any of `body / title / heading` fields are stored by the indexer to generate result previews. (Skip to 3 if present)
2. If the document has the internal `_relative_fp` field and `sourceFilesUrl` is specified, retrieve the original document (`.html` or `.json`), and transform it into the same format as that generated by the indexer.
3. Transform and highlight the field stores using the `listItemRender` set of APIs below.

| Parameter   | Description |
| ----------- | ----------- |
| `config`    | This is the *indexer* configuration. |
| `query`     | A `Query` [object](#headerrenderh-opts-query-query-htmlelement).  |
| `results`   | An array of `Result` objects.<br><br>This class exposes the `getFields()` method which returns an array of `[field name, field content]` pairs. |

You may also refer to the default implementation [here](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/searchResultTransform.ts#L369) to get an idea of how to use the API.
