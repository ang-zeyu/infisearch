# Renderers

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

<div class="alert alert-warning" role="alert">
  Certain parts of the APIs here (highlighted in <span style="color: red;">red</span>) may be particularly unstable still.
</div>

This page covers the a more advanced API, "renderers", that allows you to customise the html output structure to some degree.

Some use cases for this include:
- The default structure is not sufficient for your styling needs
- You need to attach additional event listeners to elements
- You want to override or insert additional content sourced from custom fields / static content (e.g. a footer)
- You want to change the [default use case](./search_configuration.md#default-rendering-output--purpose) of following through on a result preview to its source document entirely

If you only need to style the dropdown or search popup, you can include your own css file to do so [and / or override the variables](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/styles/search.css) exposed by the default css bundle.

These API options are similarly specified under the `uiOptions` key of the root configuration object.

```ts
initMorsels({
    uiOptions: {
        // ...
    }
});
```

As the interfaces are rather low level, this page will cross reference the `UiOptions` interface [specification](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/SearchUiOptions.ts) directly.

## The `h` function

`h`

Almost all renderer functions are passed a "`h`" function. This is an **optional** helper function you may use to create your own renderer.

The method signature is as such:

```ts
export type CreateElement = (
  // Element name
  name: string,

  // Element attribute map
  attrs: { [attrName: string]: string },

  // Child elements (HTMLElement) OR text nodes (just put the string)
  // string parameters utilise .textContent,
  // so you don't have to worry about escaping potentially malicious content
  ...children: (string | HTMLElement)[]
) => HTMLElement;
```

## Passing Custom Options

`opts`

All renderer functions are also passed an `opts` parameter. This is the original input object that you provided to the `initMorsels` call. Default parameters are however populated at this point.

i.e.,
```
opts = export interface SearchUiOptions {
  searcherOptions?: SearcherOptions,
  uiOptions?: UiOptions,
  isMobileDevice: () => boolean,
  otherOptions: ArbitraryOptions
}
```

If you want to include some custom options (e.g. an API base url) somehwere, you can use the `otherOptions` key, which is guaranteed to be untouched by morsels.

## Default Html Output Structure

You can have a look at the documentation further below on each API to understand each renderer, then refer back to the following output placement snippet to understand which renderers are responsible for which parts of the html output.

The output also varies depending on the [UI mode](./search_configuration.md#ui-mode) specified earlier. As usual, note that `dropdown` and `fullscreen` modes both apply to the `auto` mode.

<details>

<summary><strong>Renderers and their output placement</strong></summary>

```html
<!--
    dropdownRootRender - mode: 'dropdown'
    fsRootRender       - mode: 'fullscreen'
 -->

<!--
    **fsRootRender** START
    root element is a backdrop to facilitate backdrop dismiss
-->
<div class="morsels-fs-backdrop">
  
  <!-- **dropdownRootRender** START -->
  <!-- fsRootRender has an additional "morsels-fs-root" class on this element -->
  <div class="morsels-root">

    <!-- these two elements are for dropdownRootRender only -->
    <input id="morsels-search" placeholder="Search">
    <div class="morsels-input-dropdown-separator" style="display: none;"></div>

    <!--
        this element is for fsRootRender only,
        for wrapping search box & close button in a sticky header
    -->
    <div class="morsels-fs-input-button-wrapper">
        <input class="morsels-fs-input" type="text">
        <button class="morsels-input-close-fs"></button>
    </div>

    <ul class="morsels-list" style="display: none;">
    <!--
        **dropdownRootRender / fsRootRender** END
        
        NOTE: If using mode = 'target', the above ul element is
              substituted for the target element you specify
    -->

        <!-- **noResultsRender** START -->
        <div class="morsels-no-results">No results found</div>
        <!-- **noResultsRender** END -->

        <!-- **fsBlankRender** START
          Shown for the fullscreen version, when the search box is empty
        -->
        <div class="morsels-fs-blank">Powered by tiny Morsels of 🧀</div>
        <!-- **fsBlankRender** END -->

        <!--
          **loadingIndicatorRender** START (blank by default)

          Shown when making the initial search from a blank search box.
          Subsequent searches (ie. when there are some results already)
          will not show this indicator.
        -->
        <span class="morsels-loading-indicator"></span>
        <!-- **loadingIndicatorRender** END -->

        <!-- **termInfoRender** START (intentionally blank by default) -->
        <div></div>
        <!-- **termInfoRender** END -->

        <!-- results placeholder (refer to "rendering search results") -->
    </ul>
  </div>
</div>
```

</details>

You can find the latest default implementations of the renderers [here](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/search.ts).

## Root Elements

### Dropdown

`dropdownRootRender(h, opts, inputEl): { root: HTMLElement, listContainer: HTMLElement }`

This API renders the root element for the **dropdown version** of the user interface.

- `inputEl`: Input element found by the `input` configuration option

It should return 2 elements:
- `root`: The root element. This is passed to the `hide / show` APIs below.
- `listContainer`: The element to attach elements rendered by `listItemRender` (matches for a single document) to.
  - in the above snippet, this is the `<ul class="morsels-list"></ul>` element

#### Supplementary Mandatory Functions

The following two functions should be implemented **in tandem** with the above function; They are used internally to show / hide the dropdown on certain events (for example, on input focus / blur).

`showDropdown?: (root: HTMLElement, opts: SearchUiOptions) => void`

`hideDropdown?: (root: HTMLElement, opts: SearchUiOptions) => void`

For example, the default `showDropdown` implementation is as such:

```ts
(root, listContainer) => {
  if (listContainer.childElementCount) {
    listContainer.style.display = 'block';
    (listContainer.previousSibling as HTMLElement).style.display = 'block';
  }
}
```

It first checks if the `listContainer` (the dropdown), which contains result matches, has any child elements. If so, it sets `style=display:block;` on it, and its previous sibling, which is the triangle dropdown separator container.

### Fullscreen

`fsRootRender(h, opts, fsCloseHandler): { root: HTMLElement, listContainer: HTMLElement, input: HTMLInputElement }`

This API renders the root element for the **fullscreen version** of the user interface.

- `fsCloseHandler`: A void function used for closing the fullscreen UI. This may also be used to check if the current render is for the fullscreen UI or dropdown UI.

It should return 3 elements:
- `root`: The root element. This is passed to the `hide / show` APIs below.
- `listContainer`: The element to attach elements rendered by `listItemRender` (matches for a single document) to.
- `input`: Input element. This is required for morsels to attach input event handlers.

#### Supplementary Mandatory Functions

Similarly, there are two `show / hide` variants for the fullscreen version:

```ts
showFullscreen?: (
  root: HTMLElement,
  listContainer: HTMLElement,
  fullscreenContainer: HTMLElement,
  opts: SearchUiOptions,
) => void,

hideFullscreen?: (
  root: HTMLElement,
  listContainer: HTMLElement,
  fullscreenContainer: HTMLElement,
  opts: SearchUiOptions
) => void,
```

The `fullscreenContainer` (by default the `<body>` element) to which to append the root element is also provided. You may also want to for example, refocus the fullscreen version's `<input>` element once UI is shown.

### Target

There is no root element for the target, as it is specified by the `target` option. The equivalent of the `target` element would be the `listContainer` element for the dropdown / fullscreen versions  above.


## Miscellaneous Renderers

| Function        | Return | Description |
| ----- | ----- | ----------- |
| `noResultsRender(h, opts)` | `HTMLElement`        | This API renders the element attached under the `listContainer` (or the target element when using `mode = 'target'`) when there are no results found for a given query. &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;   |
| `loadingIndicatorRender(h, opts)` | `HTMLElement`  | This API renders the loading indicator attached under the `listContainer`. The loading indicator is shown when making the initial search (the first search from an empty search box).    |
| `termInfoRender(h, opts, queryParts: QueryPart[])` | `HTMLElement[]`      | This API renders element(s) attached under the `listContainer` related to the searched terms, and is blank by default.<br><br>This can be used to render messages like "*Did you mean <u>spelling</u>?* ".    |
| `fsBlankRender(h, opts)`<br><br>( `mode='fullscreen'` only ) | `HTMLElement` | This API renders the element attached under the `listContainer` when the search box is empty for the fullscreen UI.<br><br>This contrasts with the dropdown UI which is hidden in such a case.    |

### `queryParts` Parameter

This parameter to the `termInfoRender` function is the parsed structure of the input query string.

Its interface is as follows:

```ts
export interface QueryPart {
  partType: 'TERM' | 'PHRASE' | 'BRACKET' | 'AND' | 'NOT';

  // Raw, original term(s) contained, if any of the below 3 operations were applied
  originalTerms?: string[];

  isCorrected?: boolean;        // did this query part undergo spelling correction?
  isStopWordRemoved?: boolean;  // did this query part undergo stop word removal?
  isExpanded?: boolean;         // is this an added / expanded term?

  shouldExpand?: boolean;       // was this term a source for query term expansion?

  fieldName?: string;           // was a field filter applied?

  // Spelling corrected / Expanded / Stop word removed result
  terms?: string[];

  children?: QueryPart[];
}
```

## Rendering Search Results

The below **2 remaining sets of APIs** render the results for all document matches, and are **mutually exclusive** in that the second set of APIs are "building blocks" of the first (which only has one available API). So, reconfiguring the first API would invalidate any changes to the second.

Together, they are placed in the `<!-- results placeholder (refer to "rendering search results") -->` comment earlier (see [html output structure](#default-html-output-structure)).

In the following snippet, APIs belonging to the first / second are annotated with `1.` & `2.`.

<details open>

<summary><strong>Remaining renderers and their output placement</strong></summary>

```html
<!--
  **1. resultsRender** START matches for all documents
  **2. listItemRender** START A match for a single document
-->
<li class="morsels-list-item">
  <a class="morsels-link" href="http://192.168.10.132:3000/...truncated.../index.html">

    <div class="morsels-title">
      <span>
        This is the Document Title Extracted from the "title" Field
      </span>
    </div>

    <!--
      **headingBodyRender** START
      a heading and/or body field pair match for the document
    -->
    <a class="morsels-heading-body" href="http://192.168.10.132:3000/...truncated.../index.html#what">
      <!-- Sourced from the "heading" field -->
      <div class="morsels-heading"><span>What</span></div>
      <div class="morsels-bodies">
        <!--
          The text under the following element is sourced from
          the "body" field, that follows the "heading" field above
          in the original document.

          Refer to the section on indexing configuration for more details.
        -->
        <div class="morsels-body">
          <!-- (the query here is "foo bar") -->
          <span class="morsels-ellipsis"></span>

          <span> this is text before the first highlighted term </span>
          <!-- **highlightRender** START  -->
          <span class="morsels-highlight"><span>foo</span></span>
          <!-- **highlightRender** END -->
          <span> this is some text after the first highlighted term</span>


          <span class="morsels-ellipsis"></span>


          <span> this is text before the second highlighted term</span>
          <!-- **highlightRender** START -->
          <span class="morsels-highlight"><span>bar</span></span>
          <!-- **highlightRender** END -->
          <span> this is some text after the second highlighted term<< /span>

          <span class="morsels-ellipsis"></span>
        </div>
      </div>
    </a>
    <!-- **headingBodyRender** END -->

    <!--
      **bodyOnlyRender** START
      a body-only field match for the document
      (no heading before it in the original document)
    -->
    <div class="morsels-body">
      <span class="morsels-ellipsis"></span>
      <span></span>
      <!-- **highlightRender** START -->
      <span class="morsels-highlight"><span>foo</span></span>
      <!-- **highlightRender** END -->
      <span class="morsels-ellipsis"></span>
    </div>
    <!-- **bodyOnlyRender** END -->
  </a>
</li>
<!-- **listItemRender** END -->

<!--
    ... Repeat (another search result) ...

    Note: an IntersectionObserver is attached to the
    last such <li> element for infinite scrolling
-->
<li class="morsels-list-item"></li>
<!-- **resultsRender** END -->
```

</details>

<br>

### 1. Rendering All Results

`async resultsRender(h, opts, config, results, query): Promise<HTMLElement[]>`

<span style="color: red;">(flexible but not too well documented yet, and may be unstable)</span>

This API renders the results for *all* document matches.

Some examples use cases are:
- Altering the html output structure substantially (e.g. displaying results in a tabular form)
- Calling external API calls to retrieve additional info for generating result previews.
  - For this reason, this is also the only `async` API

For example, the default implementation does the following:
1. Check the `config.fieldInfos` if any of `body / title / heading` fields are stored by the indexer to generate result previews. (Skip to 3 if present)
2. If the document has the internal `_relative_fp` field and `sourceFilesUrl` is specified, retrieve the original document (`html` or `json`), and transform it into the same format as that generated by the indexer.
3. Transform and highlight the field stores using the `listItemRender` set of APIs below.

| Parameter   | Description |
| ----------- | ----------- |
| `config`    | This is the **indexing** configuration object. |
| `results`   | an array of `Result` objects |
| `query`     | a `Query` object |

You may also refer to the default implementation [here](https://github.com/ang-zeyu/morsels/blob/main/packages/search-ui/src/searchResultTransform.ts#L369) to get an idea of how to use the API.

### 2. Rendering a Single Result

The renderers under this key **build up the default implementation of `resultsRender`**, and are grouped under `uiOptions.resultsRenderOpts` (instead of `uiOptions.XXX`).
If overriding `resultsRender` above, the following options will be ignored.

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

#### 2.1 `listItemRender` supporting APIs

The remaining 3 APIs below are supplementary to `listItemRender`, and are responsible for generating the `resultTitle` and `resultHeadingsAndTexts` parameters for `listItemRender`.

Refer to the html snippet above and annotations below to understand which APIs are responsible for which parts.

```ts
interface SearchUiRenderOptions {
  // Renders a "heading" field,
  // along with the highlighted "body" fields that follow it (in document order)
  headingBodyRender?: (
    h: CreateElement,

    // Heading text
    heading: string,    

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
