# Search Configuration

All options here are provided through the `infisearch.init` function exposed by the search bundle.

There are 2 categories of options, the first related to the user interface, and the other search functionalities.

## Search UI Options

Search UI options are organised under the `uiOptions` key:

```ts
infisearch.init({
    uiOptions: {
        // ... options go here ...
    }
})
```

#### Site URL

`sourceFilesUrl`
- Example: `'/'` or `'https://www.infi-search.com'`

This option allows InfiSearch to construct a link to the page for search result previews. This is done by appending the relative file path of the indexed file.

Unless you are providing all links manually (see [Linking to other pages](./linking_to_others.md)), this URL must be provided.

#### Input Element

| Option      | Default Value | Description |
| ----------- | ----------- | ----------- |
| `input`     | `'infi-search'` | `id` of the input element or a HTML element reference |
| `inputDebounce`     | `100` | debounce time of keystrokes to the input element |
| `preprocessQuery`   | `(q) => q` | any function for preprocessing the query. Can be used to add a [field filter](./search_syntax.md#field-search) for example. |

The `input` element is required in most cases. Its behaviour depends on the UI mode.

#### UI Mode

`mode: 'auto'`

The search UI provides 4 main different behaviours.


| Mode        | Details |
| ----------- | ----------- |
| auto        | This uses the `fullscreen` mode for a [mobile device](#changing-the-mobile-device-detection-method), and `dropdown` otherwise.<br>This adjustment is rerunned whenever the window is resized.   |
| dropdown    | This wraps the provided `input` element in a wrapper container, then creates a dropdown next to house InfiSearch's UI.    |
| fullscreen  | This creates a distinct modal (with its own search input, close button, etc.) and appends it to the page `<body>`.<br><br>If the `input` element is specified, a click handler is attached to open this UI so that it functions as a button. For default keyboard accessibility, some minimal and overidable [styling](./search_configuration_styling.md#input-element) is also applied to this button.<br><br>This UI can also be toggled [programatically](#manually-showing--hiding-the-fullscreen-ui), removing the need for the `input`.    |
| target      | This option is most flexible, and is used by the mdBook plugin (this documentation).<br><br>Search results are then output to a custom `target` element of choice.    |

Use the following buttons to try out the different modes. The default in this documentation is `target`.

<style>
    .demo-btn {
        padding: 5px 9px;
        margin: 0 8px 8px 8px;
        border: 2px solid var(--sidebar-bg) !important;
        border-radius: 10px;
        transition: all 0.15s linear;
        color: var(--fg) !important;
        text-decoration: none !important;
        font-weight: 600 !important;
    }

    .demo-btn:hover {
        color: var(--sidebar-fg) !important;
        background: var(--sidebar-bg) !important;
    }

    .demo-btn:active {
        color: var(--sidebar-active) !important;
    }
</style>

<div style="display: flex; justify-content: center; flex-wrap: wrap;">
    <a class="demo-btn" href="?mode=auto">Auto</a>
    <a class="demo-btn" href="?mode=dropdown">Dropdown</a>
    <a class="demo-btn" href="?mode=fullscreen">Fullscreen</a>
    <a class="demo-btn" href="?mode=target">Target</a>
</div>

#### UI Mode Specific Options

There are also several options specific to each mode. `dropdown` and `fullscreen` options are also applicable to the `auto` mode.

| Mode        | Option                | Default                 | Description |
| ----------- | -----------           | -----------             | ----------- |
| dropdown  | `dropdownAlignment`   | `'bottom-end'`          | `'bottom'` or `'bottom-start'` or `'bottom-end'`.<br><br>The alignment will be automatically flipped horizontally to ensure optimal placement.
| fullscreen | `fsContainer`         | `<body>`        | `id` of or an element reference to attach the modal to.
| fullscreen | `fsScrollLock`        | `true` | Scroll locks the body element when the fullscreen UI is opened.
| target    | `target`              | `undefined`                       | `id` of or an element reference to attach the UI.

#### General Options

| Option                | Default                 | Description |
| -----------           | -----------             | ----------- |
| `tip`                 | `true`                  | Shows the advanced search tips icon on the bottom right.
| `maxSubMatches`       | `2`                     | Maximum headings to show for a result preview.
| `resultsPerPage`      | `10`                    | Number of results to load when the 'load more' is clicked.
| `useBreadcrumb`       | `false`                 | Prefer using the file path as the result preview's title. This is formatted into a breadcrumb, transformed to Title Case.<br><br>Example: `'documentation/userGuide/my_file.html'` → `Documentation » User Guide » My File`.

#### Setting Up Enum Filters ∀

Enum [fields](./indexer/fields.md#field-storage) you index can be mapped into UI multi-select dropdowns. In this documentation for example, Mdbook's section titles "User Guide", "Advanced" are mapped.

Setup bindings under `uiOptions` like so:

```ts
multiSelectFilters: [
  {
    fieldName: 'partTitle',
    displayName: 'Section',
    defaultOptName: 'None',
    collapsed: true,  // only the first header is initially expanded
  },
]
```

The `fieldName` corresponds to the `name` of your [field](./indexer/fields.md) definition, while `displayName` controls the UI header text.

Documents that do not have an enum value are assigned an internal default enum value. The option text of this enum value to show is specified by `defaultOptName`.

#### Setting Up Numeric Filters and Sort Orders

Indexed numeric [fields](./indexer/fields.md#field-storage) can be mapped into minimum-maximum filters of `<input type="number|date|datetime-local" />`, or used to create custom sort orders.

*Minimum-Maximum Filters*

```ts
numericFilters: [
  {
    fieldName: 'pageViewsField',
    displayName: 'Number of Views',
    type: 'number' | 'date' | 'datetime-local',
    // Text above date, datetime-local filters and placeholder text for number filters
    // Also announced to screenreaders
    minLabel: 'Min',
    maxLabel: 'Max',
  }
]
```

*Sorting by Numbers, Dates*

```ts
sortFields: {
  // Map of the name of your numeric field to names of UI options
  price: {
    asc: 'Price: Low to High',
    desc: 'Price: High to Low',
  },
},
```

#### Manually Showing / Hiding the Fullscreen UI

Call the `showFullscreen()` and `hideFullscreen()` functions returned by the `infisearch.init` to programatically show/hide the fullscreen search UI.

```ts
// These methods can be used under mode="auto|fullscreen"
const { showFullscreen, hideFullscreen } = infisearch.init({ ... });
```

#### Client Side Routing

To override the link click handler, use the specially provided parameter `onLinkClick`.

```js
uiOptions: {
  onLinkClick: function (ev) {
    /*
     By default, this function is a thunk.
     Call ev.preventDefault() and the client-side routing code here.
     Access the anchor element using "this".
    */
  }
}
```

#### Changing The Mobile Device Detection Method


If the client is a "mobile device", the fullscreen version of the user interface under `mode='auto'`.
This check is done through the following media query, which can be overwritten:

```js
uiOptions: {
  // Any function returning a boolean
  isMobileDevice: () =>
    window.matchMedia('only screen and (max-width: 768px)').matches,
}
```

## Search Functionality Options

The options regarding search functionalities itself are rather brief, its defaults are summarised in this snippet:

```js
infisearch.init({
    searcherOptions: {
        // URL of output directory generated by the CLI tool
        url: 'http://192.168.10.132:3000/output/',

        maxAutoSuffixSearchTerms: 3,
        maxSuffixSearchTerms: 5,

        useQueryTermProximity: true,

        // The maximum number of results to retrieve (unlimited if null).
        resultLimit: null,

        // ---------------------------------------------------------------
        // Caching Options

        // Whether to cache **all** the texts storage=[{ "type": "text" }] fields on initialisation,
        // to avoid making network requests when generating result previews.
        // See the chapter on Fields for more information.
        cacheAllFieldStores: undefined,

        // Any index chunk larger than this number of bytes
        // will be persistently cached once requested.
        plLazyCacheThreshold: 0,

        // ---------------------------------------------------------------
    },
});
```

#### (Automatic) Suffix Search

`maxAutoSuffixSearchTerms = 3`

Stemming is turned off by [default](./language.md#ascii-tokenizer). This does mean a bigger dictionary (but not too much usually), and lower recall, but much more precise searches.

To keep recall up, an automatic wildcard suffix search is performed on the last query term of a free text query, and only if the query does not end with a whitespace (an indicator of whether the user has finished typing).

`maxSuffixSearchTerms = 5`

This controls the maximum number of terms to search for manual wildcard [suffix searches](./search_syntax.md#wildcard-search).

#### Term Proximity Ranking

`useQueryTermProximity = true`

If positions are indexed, document scores are also scaled by how close query expressions or terms are to each other. This boosts result relevance significantly.

#### Caching Options (Advanced)

This is discussed more in the chapter on [larger collections](./larger_collections.md).
