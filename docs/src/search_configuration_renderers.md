# Renderers

This page covers the a more advanced API, "renderers", that allows you to customise the html output structure to some degree.

Some use cases for this include:
- The default structure is not sufficient for your styling needs
- You need to attach additional event listeners to elements
- You want to override or insert additional content sourced from custom fields / static content (e.g. a footer)
- You want to change the [default use case](./search_configuration.md#default-rendering-output--purpose) of following through on a result preview to its source document entirely

If you only need to style the dropdown or search popup, you can include your own css file to do so and / or override the variables exposed by the default css bundle.

These API options are specified under the `render` key of the root configuration object.

```ts
initMorsels({
    // ...
    
    render: {
        // ...
    }
});
```

<details>

<summary><strong>Typescript Interface Reference</strong></summary>

```ts
interface SearchUiRenderOptions {
    // ... some other options covered in the previous section ...

    show?: (root: HTMLElement, opts: ArbitraryRenderOptions, isPortal: boolean) => void,

    hide?: (root: HTMLElement, opts: ArbitraryRenderOptions, isPortal: boolean) => void,

    rootRender?: (
        h: CreateElement,
        opts: ArbitraryRenderOptions,
        inputEl: HTMLElement,
    ) => ({ root: HTMLElement, listContainer: HTMLElement }),

    portalRootRender?: (
        h: CreateElement,
        opts: ArbitraryRenderOptions,
        portalCloseHandler: () => void,
    ) => ({ root: HTMLElement, listContainer: HTMLElement, input: HTMLInputElement }),

    noResultsRender?: (h: CreateElement, opts: ArbitraryRenderOptions) => HTMLElement,

    portalBlankRender?: (h: CreateElement, opts: ArbitraryRenderOptions) => HTMLElement,

    loadingIndicatorRender?: (h: CreateElement, opts: ArbitraryRenderOptions) => HTMLElement,

    termInfoRender?: (
        h: CreateElement,
        opts: ArbitraryRenderOptions,
        misspelledTerms: string[],
        correctedTerms: string[],
        expandedTerms: string[],
    ) => HTMLElement[],

    resultsRender?: (
        h: CreateElement,
        initMorselsOptions: SearchUiOptions,
        config: MorselsConfig,
        results: Result[],
        query: Query,
    ) => Promise<HTMLElement[]>,

    // Options / more renderers for the default implementation of resultsRender
    resultsRenderOpts?: {
        resultsPerPage: 8,

        listItemRender?: (
            h: CreateElement,
            opts: ArbitraryRenderOptions,
            fullLink: string,
            resultTitle: string,
            resultHeadingsAndTexts: (HTMLElement | string)[],
            fields: [string, string][],
        ) => HTMLElement,

        headingBodyRender?: (
            h: CreateElement,
            opts: ArbitraryRenderOptions,
            heading: string,
            bodyHighlights: (HTMLElement | string)[],
            href?: string,
        ) => HTMLElement,

        bodyOnlyRender?: (
            h: CreateElement,
            opts: ArbitraryRenderOptions,
            bodyHighlights: (HTMLElement | string)[],
        ) => HTMLElement,

        highlightRender?: (
            h: CreateElement,
            opts: ArbitraryRenderOptions,
            matchedPart: string,
        ) => HTMLElement,
    },

    // Any options you want to pass to any of the render functions above (ArbitraryRenderOptions) from the initMorsels call
    opts?: ArbitraryRenderOptions,
}

interface ArbitraryRenderOptions {
    [key: string]: any,
    dropdownAlignment?: 'left' | 'right',
}
```

</details>


## The `h` function

All renderer functions are passed a "`h`" function. This is an optional helper function you may use to create your own renderer.

The method signature is as such:

```ts
export type CreateElement = (
  // Element name
  name: string,

  // Element attribute map
  attrs: { [attrName: string]: string },

  // Child elements (HTMLElement) OR text nodes (string)
  // string parameters utilise .textContent,
  // so you don't have to worry about escaping potentially malicious content
  ...children: (string | HTMLElement)[]
) => HTMLElement;
```

## Passing Custom Options

All renderer functions are also passed an `opts` parameter. This provided in the `render.opts` key, and basically allows passing in any additional initilisation-time options you might need for the renderers. (e.g. an API url)

## Default Html Output Structure

Have a look at the following snippet when reading the documentation below on each API to understand which renderers (bracketed on the left of each comment) are responsible for which parts of the html output by default.

Note that there are some minor differences between the dropdown version and fullscreen version, also annotated below.

<details>

<summary><strong>Renderers and their output placement</strong></summary>

```html
<!-- (rootRender / portalRootRender) START -->

<!--
    portalRootRender only
    root element is a backdrop to facilitate backdrop dismiss
-->
<div class="morsels-portal-backdrop">
<!-- fullscreen version end -->

<!-- Note: fullscreen version has an additional "morsels-portal-root" class -->
<div class="morsels-root">

    <!-- rootRender (dropdown) only -->
    <input id="morsels-search" placeholder="Search">
    <div class="morsels-input-dropdown-separator" style="display: none;"></div>
    <!-- rootRender end -->

    <!-- portalRootRender only, wrap search box & close button in a sticky header -->
    <div class="morsels-portal-input-button-wrapper">
        <input class="morsels-portal-input" type="text">
        <button class="morsels-input-close-portal"></button>
    </div>
    <!-- portalRootRender end -->

    <ul class="morsels-list" style="display: none;">
<!-- (rootRender / portalRootRender) END -->

        <!-- (noResultsRender) START -->
        <div class="morsels-no-results">No results found</div>
        <!-- (noResultsRender) END -->

        <!-- (portalBlankRender) START
          Shown for the fullscreen version, when the search box is empty
        -->
        <div class="morsels-portal-blank">Powered by tiny Morsels of 🧀</div>
        <!-- (portalBlankRender) END -->

        <!-- (loadingIndicatorRender) START (blank by default)
          Shown when making the initial search from a blank search box.
          Subsequent searches (ie. when there are some results already)
          will not show this indicator.
        -->
        <span class="morsels-loading-indicator"></span>
        <!-- (loadingIndicatorRender) END -->

        <!-- (termInfoRender) START (blank by default) -->
        <div></div>
        <!-- (termInfoRender) END -->

        <!-- results placeholder (refer to "rendering search results") -->
    </ul>
</div>
    
</div>
```

</details>

## Rendering the Root Elements

**`rootRender(h, opts, inputEl): { root: HTMLElement, listContainer: HTMLElement }`**

This API renders the root element for the **dropdown version** of the user interface.

- `inputEl`: Input element found by the `inputId` configuration

It should return 2 elements:
- `root`: The root element. This is passed to the `hide / show` APIs below.
- `listContainer`: The element to attach elements rendered by `listItemRender` (matches for a single document) to.

---

**`portalRootRender(h, opts, portalCloseHandler): { root: HTMLElement, listContainer: HTMLElement, input: HTMLInputElement }`**

This API renders the root element for the **fullscreen version** of the user interface.

- `portalCloseHandler`: A void function used for closing the fullscreen UI. This may also be used to check if the current render is for the fullscreen UI or dropdown UI.

It should return 3 elements:
- `root`: The root element. This is passed to the `hide / show` APIs below.
- `listContainer`: The element to attach elements rendered by `listItemRender` (matches for a single document) to.
- `input`: Input element. This is required for morsels to attach input event handlers.

---

**`hide / show (root, opts, isPortal): void`**

These two APIs are not responsible for html output, but rather, hiding and showing the fullscreen or dropdown UIs (e.g. via `style="display: none"`).

- `root`: root element returned by `rootRender`
- `isPortal`: whether the function call is for the fullscreen / dropdown UI version

## Miscellaneous Renderers

**`noResultsRender(h, opts): HTMLElement`**

This API renders the element attached under the `listContainer` when there are no results found for a given query.

---

**`portalBlankRender(h, opts): HTMLElement`**

This API renders the element attached under the `listContainer` when the search box is empty for the fullscreen UI.

The dropdown UI is hidden in such a case.

---

**`loadingIndicatorRender(h, opts): HTMLElement`**

This API renders the loading indicator attached under the `listContainer`. The loading indicator is shown when making the initial search (the first search from an empty search box).

---

**`termInfoRender(h, opts, misspelledTerms, correctedTerms, expandedTerms): HTMLElement[]`**

This API renders elements attached under the `listContainer` related to the searched terms, and is blank by default.

For example, you may render `<div>Did you mean <u>corrected</u>?</div>` for the misspelled query `correkted`.

## Rendering Search Results

The below **2 remaining sets of APIs** render the results for all document matches, and are **mutually exclusive**. Use only one or the other.

Together, they are placed in the `<!-- results placeholder (refer to "rendering search results") -->` (see [html output structure](#default-html-output-structure)).

<details>

<summary><strong>Remaining renderers and their output placement</strong></summary>

```html
<!-- (resultsRender) START matches for **all documents** -->
<!-- (listItemRender) START A match for a **single document** -->
<li class="morsels-list-item">
    <a
        class="morsels-link"
        href="http://192.168.10.132:3000/...truncated.../index.html"
    >

        <div class="morsels-title">
            <span>
                This is the Document Title Extracted from the "title" Field
            </span>
        </div>

        <!-- (headingBodyRender) START
            a heading and/or body field pair match for the document
        -->
        <a
            class="morsels-heading-body"
            href="http://192.168.10.132:3000/...truncated.../index.html#what"
        >
            <!--
                Sourced from the "heading" field
            -->
            <div class="morsels-heading"><span>What</span></div>
            <div class="morsels-bodies">
                <!--
                    The text under the following element is sourced from
                    the "body" field, and follows the "heading" field above
                    in the original document.
                -->
                <div class="morsels-body">
                    <span class="morsels-ellipsis"></span>
                    <span> this is text before the first highlighted term </span>
                    <!-- (highlightRender) START (the query is "foo bar") -->
                    <span class="morsels-highlight"><span>foo</span></span>
                    <!-- (highlightRender) END -->
                    <span> this is some text after the first highlighted term</span>
                    <span class="morsels-ellipsis"></span>
                    <span> this is text before the second highlighted term</span>
                    <!-- (highlightRender) START (the query is "foo bar") -->
                    <span class="morsels-highlight"><span>bar</span></span>
                    <!-- (highlightRender) END -->
                    <span> this is some text after the second highlighted term<</span>
                    <span class="morsels-ellipsis"></span>
                </div>
            </div>
        </a>
        <!-- (headingBodyRender) END -->

        <!-- (bodyOnlyRender) START
            
            a body-only field match for the document
            (it does not have a heading before it in the original document)
        -->
        <div class="morsels-body">
            <span class="morsels-ellipsis"></span>
            <span></span>
            <!-- (highlightRender) START -->
            <span class="morsels-highlight"><span>foo</span></span>
            <!-- (highlightRender) END -->
            <span class="morsels-ellipsis"></span>
        </div>
        <!-- (bodyOnlyRender) END -->
    </a>
</li>
<!-- (listItemRender) END -->

<!--
    Note: an IntersectionObserver is attached to the
    last such <li> element for infinite scrolling
-->
<li class="morsels-list-item">
    <!-- Another search result -->
</li>
<!-- (resultsRender) END -->
```

</details>

<br>

**1. `async resultsRender(h, initMorselsOptions, config, results, query)`** <span style="color: red">(advanced)</span>

This API renders the results for *all* document matches.

This can be used for example, if the output required is substantially different or external API calls are required to retrieve document info.

For example, the default implementation does the following:
1. Check the `config.fieldInfos` if any of `body / title / heading` fields are stored by the indexer to generate result previews. (Skip to 3 if present)
2. If the document has the internal `_relative_fp` field and `sourceFilesUrl` is specified, retrieve the original document (`html` or `json`), and transform it into the same format as that generated by the indexer.
3. Transform and highlight the field stores using the `listItemRender` set of APIs below.

Refer to the default implementation [here](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/searchResultTransform.ts#L369) to get an idea of how to use the API.

---

**2. `resultsRenderOpts`**

The renderers and options under this key are based on the **default implementation of `resultsRender`**.
If overriding `resultsRender` above, the following options will be ignored.

<br>

**2.1 `resultsPerPage = 8`**

This option controls how many result previews are generated per trigger of the infinite scrolling intersection observer.

If none of the `body / title / heading` fields are stored, lowering this has a noticeable performance improvement on result generation, as more `.html / .json` files have to be retrieved on-the-fly, parsed, and processed.

<br>

**2.2 `listItemRender(h, fullLink, resultTitle, resultHeadingsAndTexts, fields)`**

This API renders the result for a single document match.

- `fullLink` - full resource link of the document
- `resultTitle` - extracted `title` field of the document, if any
- `resultHeadingsAndTexts` - An array of strings & html elements intended to be used as the last parameter of `h`. This contains the highlighted heading-body pair matches or body only matches rendered from the below 2 apis.
- `fields` - All stored fields for the document, as positioned `[fieldName, fieldValue]` pairs. Useful if adding additional fields.

The following example shows the default implementation, and how to add an additional field, `subtitle`, to each result.

```ts
const subTitleField = fields.find(field => field[0] === 'subtitle');

const linkEl = h('a', { class: 'morsels-link' },
  h('div', { class: 'morsels-title' }, title,
    h('div', { class: 'morsels-subtitle' }, (subTitleField && subTitleField[1]) || '')
  ),
  ...bodies);
if (fullLink) {
  linkEl.setAttribute('href', fullLink);
}

return h(
  'li', { class: 'morsels-list-item' },
  linkEl,
);
```

<br>

**2.3 `listItemRender` supporting APIs**

The remaining 3 APIs are supplementary to `listItemRender`, and are responsible for generating the `resultTitle` and `resultHeadingsAndTexts` parameters.

Refer to the html snippet above and annotations below to understand which APIs are responsible for which parts.

```ts
interface SearchUiRenderOptions {
    // ...
    headingBodyRender?: (
        h: CreateElement,

        // Heading text
        heading: string,    

        // The elements under .morsels-body. Intended to be used with the 'h' function.
        bodyHighlights: (HTMLElement | string)[], 

        // Url of the document + The matching heading's id, if any
        href?: string                             
    ) => HTMLElement,
    bodyOnlyRender?: (
        h: CreateElement,

        // The elements under .morsels-body. Intended to be used with the 'h' function.
        bodyHighlights: (HTMLElement | string)[], 
    ) => HTMLElement,
    highlightRender?: (
        h: CreateElement,

        // matched term
        matchedPart: string,                      
    ) => HTMLElement,
}
```
