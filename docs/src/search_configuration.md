# Search Configuration

All search time related options can be provided through the `initMorsels` function, exposed by `@morsels/search-ui`.

```js
initMorsels({
    // Options belonging to @morsels/search-lib, the search library package
    searcherOptions: {
        // Base url of output directory from cli tool
        url: 'http://192.168.10.132:3000/output',
        
        // Whether to use substring query term expansion
        numberOfExpandedTerms: 3,
        
        // Override for using query term proximity ranking or not. Disabled for mobile devices by default
        useQueryTermProximity: true,
    },
    
    // Options for @morsels/search-ui, the user interface package
    
    // Id of input element to attach the search dropdown to
    inputId: 'morsels-search',
    
    // Mandatory, by default - base url for sourcing .html / .json files
    sourceFilesUrl: 'http://192.168.10.132:3000/source',
    
    // Customise search result outputs, UI behaviour
    render: {
        // refer to Renderers section
    }
});
```

## Mobile Device Detection

Mobile devices are detected through a simple `window.matchMedia('only screen and (max-width: 1024px)').matches` query at initialisation time, which may not be accurate.

This is only used for some default settings, such as deciding whether to use query term proximity ranking. Overrides may be provided through the options above if this detection method is inadequate.

## url

This parameter must be specified, and tells Morsels where to find the output from the indexer.

## numberOfExpandedTerms

By default, stemming is turned off in the [language modules](indexing_configuration.md#language). This does mean a bigger dictionary (but not that much anyway), and lower recall, but much more precise searches.

To provide a compromise for recall, query terms that are similar to the searched term are added to the query, although with a lower weight.

This is only applied for the last term (if any) of a query, and if the query string immediately ends with that term (e.g. no whitespace after it).
You may also think of this as implicit wildcard search for the last query term.

## useQueryTermProximity

Document scores are also scaled by how close query expressions or terms are to each other, if positions are indexed.
This may be costly for mobile devices however, and is disabled by default in such cases.

## sourceFilesUrl

This option is only applicable for indexed html and json files.

You will need to specify this in the default setup, and / or if you did not specify `do_store: true` for any of the necessary fields listed [here](./indexing_configuration.md#fields-needed-for-morselssearch-ui).

This parameter tells search-ui where to find the **source** files that were indexed by the cli tool, so that it may generate result previews from them.

If `do_store: true` is specified for the fields required by search-ui detailed in the indexing configuration page, then morsels' internal json field store will be used instead.

You may wish to use `do_store: true` over this parameter if filesystem bloat isn't too much of a concern. Apart from avoiding the additional http requests, the internal json field store comes packed in a format that is more performant for search-ui to perform result preview generation on.

## Customising UI Behaviour & Output

The html output and UI behaviour can also be customised to some degree, under the `render` configuration key.

Some use cases for this include:
- The default structure is not sufficient for your styling needs
- You need to attach additional event listeners to elements 
- You want to override or insert additional content sourced from custom fields / static content (e.g. a footer).

If you only need to theme the dropdown or search popup, you can include your own css file to do so, or override the variables exposed by the default css bundle.

### Switching between fullscreen popup or dropdown UI versions

The search UI provides 2 different interfaces, a dropdown and fullscreen / "popup" version.

The dropdown version is used for desktop devices by default, which is attached to the element specified by `inputId`.
For mobile devices, the fullscreen popup version is used instead.

This section details the available options to customise this behaviour.

```ts
interface SearchUiRenderOptions {
    enablePortal?: boolean | 'auto',
    portalTo?: HTMLElement,
    // ...
}
```

**`enablePortal = 'auto'`**

This parameter tells search-ui whether to use the fullscreen search UI for mobile devices when the original `<input>` is focused.

The default value of `'auto'` configures this according to mobile device detection (`true` if it is a mobile device), and also adds a simple window resize handler to automatically hide the corresponding UI if the window is resized.

You can set this to `true` / `false` to always prefer the fullscreen or dropdown version instead when the original `<input>` is focused.

**`portalTo = document.getElementsByTagName('body')[0]`**

This parameter tells morsels which element to attach the fullscreen search UI to, which uses `fixed` positioning.

**`showPortalUI(): void`**

```ts
const { showPortalUI } = initMorsels(/* ... */);
```

The default behaviour of showing the fullscreen search UI may be insufficient in some cases, for example showing the UI when clicking a search icon.

You may call the `showPortalUI()` function returned by the initMorsels call in such a case for manual control.

### Renderers

The other properties under the `render` key allow you to customise the html output structure to some degree.

```ts
interface SearchUiRenderOptions {
    // ...
    show?: (root: HTMLElement, opts: ArbitraryRenderOptions, isPortal: boolean) => void,
    hide?: (root: HTMLElement, opts: ArbitraryRenderOptions, isPortal: boolean) => void,
    rootRender?: (
        h: CreateElement,
        opts: ArbitraryRenderOptions,
        inputEl: HTMLElement,
        portalCloseHandler?: () => void,
    ) => ({ root: HTMLElement, listContainer: HTMLElement }),
    portalInputRender?: (h: CreateElement, opts: ArbitraryRenderOptions) => HTMLInputElement,
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
    }
    // Any options you want to pass to any of the render functions below, from the initMorsels call
    opts?: ArbitraryRenderOptions,
}

interface ArbitraryRenderOptions {
    [key: string]: any,
    dropdownAlignment?: 'left' | 'right',
}
```

---

**The `h` function**

The `h` function is an optional helper function you may use to create your own renderer.
The signature is as such:

```ts
export type CreateElement = (
  // Element name
  name: string,

  // Element attribute map
  attrs: { [attrName: string]: string },

  // Child elements (HTMLElement) OR text (string) nodes
  // string parameters utilise .textContent,
  // so you don't have to worry about escaping potentially malicious content
  ...children: (string | HTMLElement)[]
) => HTMLElement;
```

---

**Default Html Output Structure**

Have a look at the following snippet when reading the documentation below on each API to understand which renderers (bracketed on the left of each comment) are responsible for which parts of the html output by default.

Note that there are some minor differences between the dropdown version and fullscreen version, also annotated below.

```html
<!-- (rootRender) START -->

<!-- fullscreen version only - root element is a backdrop to facilitate backdrop dismiss -->
<div class="morsels-portal-backdrop">
<!-- fullscreen version end -->
    
<div class="morsels-root"> <!-- fullscreen version has an additional "morsels-portal-root" class -->

    <!-- dropdown version -->
    <input id="morsels-search" placeholder="Search">
    <div class="morsels-input-dropdown-separator" style="display: none;"></div>
    <!-- dropdown version end -->

    <!-- fullscreen version only, wrap search box & close button in a sticky header -->
    <div class="morsels-portal-input-button-wrapper">
        <!-- (portalInputRender) START -->
        <input class="morsels-portal-input" type="text">
        <!-- (portalInputRender) END -->
        <button class="morsels-input-close-portal"></button>
    </div>
    <!-- fullscreen version end -->

    <ul class="morsels-list" style="display: none;">
<!-- (rootRender) END -->

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
          Subsequent searches (ie. when there are some results already) will not show this indicator.
        -->
        <span class="morsels-loading-indicator"></span>
        <!-- (loadingIndicatorRender) END -->

        <!-- (termInfoRender) START (blank by default) -->
        <div></div>
        <!-- (termInfoRender) END -->

        <!-- (resultsRender) START matches for **all documents** -->
        <!-- (listItemRender) START A match for a **single document** -->
        <li class="morsels-list-item">
            <a class="morsels-link" href="http://192.168.10.132:3000/source/book/testing/testingTypes/integrationTesting/index.html">

                <div class="morsels-title"><span>CS2103/T Website - Testing: Testing Types: Integration Testing</span></div>

                <!-- (headingBodyRender) START a heading and/or body field pair match for the document -->
                <a class="morsels-heading-body" href="http://192.168.10.132:3000/source/book/testing/testingTypes/integrationTesting/index.html#what">
                    <div class="morsels-heading"><span>What</span></div>
                    <div class="morsels-bodies">
                        <div class="morsels-body">
                            <span class="morsels-ellipsis"></span>
                            <span> this is text before the first highlighted term </span>
                            <!-- (highlightRender) START (the query is "software engine") -->
                            <span class="morsels-highlight"><span>software</span></span>
                            <!-- (highlightRender) END -->
                            <span> this is some text after the first highlighted term</span>
                            <span class="morsels-ellipsis"></span>
                            <span> this is text before the second highlighted term</span>
                            <!-- (highlightRender) START (the query is "software engine") -->
                            <span class="morsels-highlight"><span>engine</span></span>
                            <!-- (highlightRender) END -->
                            <span> this is some text after the second highlighted term<</span><span> ...</span>
                        </div>
                    </div>
                </a>
                <!-- (headingBodyRender) END -->

                <!-- (bodyOnlyRender) START a body-only field match for the document
                  i.e., This match does not have a corresponding heading before it / it belongs under
                -->
                <div class="morsels-body">
                    <span class="morsels-ellipsis"></span>
                    <span></span>
                    <!-- (highlightRender) START -->
                    <span class="morsels-highlight"><span>software</span></span><span> Engineering</span>
                    <!-- (highlightRender) END -->
                    <span class="morsels-ellipsis"></span>
                </div>
                <!-- (bodyOnlyRender) END -->
            </a>
        </li>
        <!-- (listItemRender) END -->

        <!-- Another document match -->
        <!--
          Note that an IntersectionObserver is attached to the
          last such <li> element for infinite scrolling
        -->
        <li class="morsels-list-item">...</li>
        <!-- (resultsRender) END -->
    </ul>
</div>
    
</div>
```

**`rootRender(h, opts, inputEl, portalCloseHandler): { root: HTMLElement, listContainer: HTMLElement }`**

- `inputEl`: Input element found by the `inputId` configuration, or created from the `portalInputRender` API below
- `portalCloseHandler`: A void function used for closing the fullscreen UI. This may also be used to check if the current render is for the fullscreen UI or dropdown UI.

It should return two elements:
- `root`: The root element. This is passed to the `hide / show` APIs below.
- `listContainer`: The element to attach elements rendered by `listItemRender` (matches for a single document) to.

**`hide / show (root, opts, isPortal): void`**

These two APIs are not responsible for html output, but rather, hiding and showing the fullscreen or dropdown UIs (e.g. via `style="display: none"`).

- `root`: root element returned by `rootRender`
- `isPortal`: whether the function call is for the fullscreen / dropdown UI version

**`portalInputRender(h, opts): HTMLInputElement`**

This API renders the new `<input>` element wen using the fullscreen UI.

**`noResultsRender(h, opts): HTMLElement`**

This API renders the element attached under the `listContainer` when there are no results found for a given query.

**`portalBlankRender(h, opts): HTMLElement`**

This API renders the element attached under the `listContainer` when the search box is empty for the fullscreen UI.

The dropdown UI is hidden in such a case.

**`loadingIndicatorRender(h, opts): HTMLElement`**

This API renders the loading indicator attached under the `listContainer`. The loading indicator is shown when making the initial search (the first search from an empty search box).

**`termInfoRender(h, opts, misspelledTerms, correctedTerms, expandedTerms): HTMLElement[]`**

This API renders elements attached under the `listContainer` related to the searched terms, and is blank by default.

For example, you may render `<div>Did you mean <u>corrected</u>?</div>` for the misspelled query `correkted`.

---

The 2 remaining sets of APIs are mutually exclusive. Use only one or the other.

**1. `async resultsRender(h, initMorselsOptions, config, results, query)`** <span style="color: red">(advanced)</span>

This API renders the results for all document matches.

Note that **the APIs below** (`listItemRender / headingBodyRender / bodyOnlyRender / highlightRender`) are **built upon the default implementation of this API**, and will be unavailable if this API is overwritten.

This can be used for example, if the output required is substantially different or external API calls are required to retrieve document info.

For example, the default implementation does the following:
1. Check the `config.fieldInfos` if any of `body / title / heading` fields are stored by the indexer to generate result previews. (Skip to 3 if present)
2. If the document has the internal `_relative_fp` field and `sourceFilesUrl` is specified, retrieve the original document (`html` or `json`), and transform it into the same format as that generated by the indexer.
3. Transform and highlight the field stores using the `listItemRender` set of APIs below.

<br>

**2. `resultsRenderOpts`**

The renderers and options under this key are based on the **default implementation** of `resultsRender`.
If overriding `resultsRender` above, these options will not be used.

**`resultsPerPage = 8`**

This option controls how many result previews are generated per trigger of the infinite scrolling intersection observer.

If none of the `body / title / heading` fields are stored, lowering this has a noticeable performance improvement on result generation, as more `.html / .json` files have to be retrieved on-the-fly, parsed, and processed.

**`listItemRender(h, fullLink, resultTitle, resultHeadingsAndTexts, fields)`** & co.

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

The remaining 3 APIs are supplementary to `listItemRender`, and are responsible for generating the `resultTitle` and `resultHeadingsAndTexts` parameters.

Refer to the html snippet above and annotations below to understand which APIs are responsible for which parts.

```ts
interface SearchUiRenderOptions {
    // ...
    headingBodyRender?: (
        h: CreateElement,
        heading: string,                          // Heading text
        bodyHighlights: (HTMLElement | string)[], // The elements under .morsels-body. Intended to be used with the 'h' function.
        href?: string                             // Url of the document + The matching heading's id, if any
    ) => HTMLElement,
    bodyOnlyRender?: (
        h: CreateElement,
        bodyHighlights: (HTMLElement | string)[], // The elements under .morsels-body. Intended to be used with the 'h' function.
    ) => HTMLElement,
    highlightRender?: (
        h: CreateElement,
        matchedPart: string,                      // matched term
    ) => HTMLElement,
}
```
