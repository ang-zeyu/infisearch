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

This page covers some APIs that allow you to customise some small parts of the HTML output structure.

Some use cases for this include:
- The default HTML structure is not sufficient for your styling needs
- You need to attach additional event listeners to elements
- You want to override or insert additional content sourced from custom fields / static content (e.g. a footer)
- You want to change the [default use case](#1-rendering-a-single-result) of linking to a web page entirely

> 💡 If you only need to style the dropdown or search popup, you can include your own css file to do so [and / or override the variables](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/styles/search.css) exposed by the default css bundle.

These API options are similarly specified under the `uiOptions` key of the root configuration object.

```ts
morsels.initMorsels({
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
   Child elements (HTMLElement) OR text nodes (string)
   string parameters utilise .textContent,
   so you don't have to worry about escaping malicious content
  */
  ...children: (string | HTMLElement)[]
) => HTMLElement;
```

## Passing Custom Options

`opts`

All renderer functions are also passed an `opts` parameter. This is the original input object that you provided to the `morsels.initMorsels` call, with default parameters populated at this point.

```ts
opts = export interface Options {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
  otherOptions: any
}
```

If you need to include some custom options (e.g. an API base url), you can use the `otherOptions` key, which is guaranteed to be untouched by morsels.


## Changing Supporting Parts of the UI

The options here are intended for changing small, supporting elements, which can be especially useful for localizing the UI.

#### `headerRender(h, opts, err: boolean, blank: boolean, queryParts: Query): HTMLElement`

This method renders:
- the "10 results found" text
- the *Start searching above!* text when the search box is empty in the fullscreen UI. The dropdown UI is hidden in such a case.
- a generic error message when an unexpected error occurs.

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

## Rendering Search Results

If you haven't, you should read through the [Search API](./search_api.md) documentation on the `Result` and `Query` parameters.

This last API allows changing the rendering of each document match. One example use case would be to call an external API to retrieve more UI data.

```ts
type ListItemRender = async (
  h: CreateElement,
  opts: Options,
  result: Result,
  query: Query,
) => Promise<HTMLElement>;
```

See the [source](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/searchResultTransform/listItemRender.ts) to get an idea of using this API.

At the current, this API is moderately lengthy, performing things such as limiting the number of sub matches (heading-body pairs) per document, formatting the relative file path of documents into a breadcrumb form, etc.

There may be room for breaking this API down further as such, please help to bring up a feature request if you have any suggestions!.
