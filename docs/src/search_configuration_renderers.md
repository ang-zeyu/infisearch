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

This page covers customising the result preview HTML output structure.

Some use cases for this include:
- The default HTML structure is not sufficient for your styling needs
- You need to attach additional event listeners to elements
- You want to override or insert additional content sourced from your own fields (e.g. an image)
- You want to change the [default use case](#1-rendering-a-single-result) of linking to a web page entirely (e.g. use client side routing)

> 💡 If you only need to style the dropdown or search popup, you can include your own css file to do so [and / or override the variables](https://github.com/ang-zeyu/infisearch/blob/main/packages/search-ui/src/styles/search.css) exposed by the default css bundle.

These API options are similarly specified under the `uiOptions` key of the root configuration object.

```ts
infisearch.init({
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

All renderer functions are also passed an `opts` parameter. This is the original input object that you provided to the `infisearch.init` call, with default parameters populated at this point.

```ts
opts = export interface Options {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
  otherOptions: any
}
```

If you need to include some custom options (e.g. an API base url), you can use the `otherOptions` key, which is guaranteed to be untouched by InfiSearch.

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

See the [source](https://github.com/ang-zeyu/infisearch/blob/main/packages/search-ui/src/searchResultTransform/listItemRender.ts) to get an idea of using this API.

**Accessibility and User Interaction**

To ensure that [combobox](https://www.w3.org/WAI/ARIA/apg/example-index/combobox/combobox-autocomplete-list.html) controls work as expected, you should also ensure that the appropriate elements are labelled with `role='option'` (and optionally `role='group'`).

Elements with `role='option'` will also have the `.focus` class applied to them once they are visually focused. You can use this class to style the option.

**Granularity**

At the current, this API is moderately lengthy, performing things such as limiting the number of sub matches (heading-body pairs) per document, formatting the relative file path of documents into a breadcrumb form, etc.

There may be room for breaking this API down further as such, please help to bring up a feature request if you have any suggestions!.
