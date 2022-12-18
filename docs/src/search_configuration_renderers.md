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
- You want to override or insert additional content sourced from your own fields (e.g. an image)
- You want to change the [default use case](#1-rendering-a-single-result) of linking to a web page entirely (e.g. use client side routing)

> 💡 If you only need to style the dropdown or search popup, you can include your own css file to do so [and / or override the variables](https://github.com/ang-zeyu/infisearch/blob/main/packages/search-ui/src/styles/search.css) exposed by the default css bundle.

The only API option is similarly specified under the `uiOptions` key of the root configuration object.

```ts
infisearch.init({
    uiOptions: {
        listItemRender: ...
    }
});
```

It's interface is as follows:

```ts
type ListItemRender = async (
  h: CreateElement,
  opts: Options,  // what you passed to infisearch.init
  result: Result,
  query: Query,
) => Promise<HTMLElement>;
```

If you haven't, you should also read through the [Search API](./search_api.md) documentation on the `Result` and `Query` parameters.


**`h` function**

This is an *optional* helper function you may use to create elements.

The method signature is as such:

```ts
export type CreateElement = (
  // Element name
  name: string,

  // Element attribute map
  attrs: { [attrName: string]: string },

  /*
   Child elements (HTMLElement) OR text nodes (string)
   String parameters are automatically escaped.
  */
  ...children: (string | HTMLElement)[]
) => HTMLElement;
```

**Accessibility and User Interaction**

To ensure that [combobox](https://www.w3.org/WAI/ARIA/apg/example-index/combobox/combobox-autocomplete-list.html) controls work as expected, you should also ensure that the appropriate elements are labelled with `role='option'` (and optionally `role='group'`).

Elements with `role='option'` will also have the `.focus` class applied to them once they are visually focused. You can use this class to style the option.

**Granularity**

At the current, this API is moderately lengthy, performing things such as limiting the number of sub matches (heading-body pairs) per document, formatting the relative file path of documents into a breadcrumb form, etc.

There may be room for breaking this API down further as such, please help to bring up a feature request if you have any suggestions!.

**Source Code**

See the [source](https://github.com/ang-zeyu/infisearch/blob/main/packages/search-ui/src/searchResultTransform/listItemRender.ts) to get a better idea of using this API.
