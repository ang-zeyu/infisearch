# Search Configuration

All options here are provided through the `initMorsels` function exposed by the search bundle.

There are 2 categories of options, the first related to the user interface, and the other search functionalities.

## Search UI Options

Search UI options are organised under the `uiOptions` key:

```ts
initMorsels({
    uiOptions: {
        // ... options go here ...
    }
})
```

#### Base URL

`sourceFilesUrl`
- Example: `'/'` or `'https://www.morsels-search.com'`

This option allows Morsels to construct the default link used in search result previews by appending the relative file path of the indexed files.

Unless you are providing all links manually (see [Linking to other pages](./linking_to_others.md)), this URL must be provided.

#### Input Element

| Option      | Default Value | Description |
| ----------- | ----------- | ----------- |
| `input`     | `'morsels-search'` | `id` of the input element or a HTML element reference |
| `inputDebounce`     | `100` | debounce time of keystrokes to the input element |
| `preprocessQuery`   | `(q) => q` | any function for preprocessing the query. Can be used to add a [field filter](./search_features.md#field-search) for example. |

The `input` element is required in most cases. Its behaviour depends on the UI mode.

#### UI Mode

`mode: 'auto'`

The search UI provides 4 main different behaviours.


| Mode        | Details |
| ----------- | ----------- |
| auto        | This option uses the `fullscreen` mode if a mobile device is [detected](#changing-the-mobile-device-detection-method). Otherwise, the `dropdown` mode is used.<br><br>An event handler is also attached that reruns this adjustment whenever the window is resized.   |
| dropdown    | This wraps the provided `input` element in a wrapper container, then places search results in a dropdown container next to it.    |
| fullscreen  | This option creates a completely distinct modal (with its own search input, close button, etc.), and attaches it to the `<body>` element.<br><br>If the `input` element is specified, a click handler is attached to open this UI. For keyboard accessibility, some minimal, but overidable [styling](./search_configuration_styling.md#input-element) is also applied to the input element.<br><br>This UI can also be shown/hidden [programatically](#manually-showing--hiding-the-fullscreen-ui), which is also *the only case* you would not need to specify the `input` element.    |
| target      | This option is the most flexible, and is used by the mdBook plugin (this documentation). The `input` element must be specified, where keystroke event listeners are attached.<br><br>Search results are then output to a custom `target` element of choice.    |

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

There are also several options specific to each mode. Note that `dropdown` and `fullscreen` options are both applicable to the `auto` mode.

| Mode        | Option                | Default                 | Description |
| ----------- | -----------           | -----------             | ----------- |
| dropdown  | `dropdownAlignment`   | `'bottom-end'`          | `'bottom'` or `'bottom-start'` or `'bottom-end'`.<br><br>This is the side of the input element to align the dropdown results container and dropdown seperator against.<br><br>The alignment will also be automatically flipped horizontally to ensure the most optimal placement.
| auto      | `fsInputButtonText`        | `undefined`| Placeholder override for the `input` if the fullscreen UI is in use.<br><br>This is added for keyboard [accessibility](./search_configuration_styling.md#styling-the-fullscreen-ui-input-button).
| fullscreen | `fsInputLabel`        | `'Search'` | Accessibility label for the original input element, when the fullscreen UI is in use.
| fullscreen | `fsContainer`         | `<body>` element        | `id` of the element, or an element reference to attach the separate root container to.
| fullscreen | `fsPlaceholder`       | `'Search this site'` | Placeholder of the input element in the fullscreen UI.
| fullscreen | `fsCloseText`         | `'Close'` | Text for the <kbd>Close</kbd> button.
| fullscreen | `fsScrollLock`        | `true` | Whether to automatically scroll lock the body element when the fullscreen UI is opened.
| all except target         | `tip`                 | `true`        | Whether to show the tip icon. When hovered over, this shows advanced usage information (e.g. how to perform phrase queries).
| target    | `target`              | `undefined`                       | `id` of the element, or an element reference to attach results to.<br><br>Required if using `mode='target'`.

#### General Options

| Option                | Default                 | Description |
| -----------           | -----------             | ----------- |
| `label`               | `'Search this site'`    | Placeholder for the fullscreen UI input.
| `resultsLabel`        | `'Site results'`        | Accessibility label for result [`listbox`](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/listbox_role).
| `useBreadcrumb`       | `false`                 | Prefer the file path of the indexed file for the result preview's title. This is formatted into a breadcrumb, with its components transformed to Title Case.<br><br>For example, `documentation/userGuide/my_file.html` is displayed as `Documentation » User Guide » My File`.
| `maxSubMatches`       | `2`                     | Maximum number of heading-body pairs to show for a document.
| `resultsPerPage`      | `8`                     | An infinite scroll intersection observer is attached to the last search result. When triggered, the next few result previews are fetched and generated .

#### Manually Showing / Hiding the Fullscreen UI

```ts
const { showFullscreen, hideFullscreen } = initMorsels({ ... });
```

You may call the `showFullscreen()` function returned by the initMorsels call to programatically show the fullscreen search UI.

Correspondingly, the `hideFullscreen()` method hides the fullscreen interface, although, this shouldn't be needed since a close button is available by default (the <kbd>Esc</kbd> key works too).

These methods can also be used under `mode="auto"`.

#### Changing The Mobile Device Detection Method


If the client is a "mobile device", the fullscreen version of the user interface is used for `mode='auto'`.

This check is done through a simple media query, which may not be adequate for your use case.

```js
window.matchMedia('only screen and (max-width: 768px)').matches
```

Use the `isMobileDevice` option to the override this check:

```js
initMorsels({
    uiOptions: {
        // Any function returning a boolean
        isMobileDevice: () => true,
    }
})
```

## Search Functionality Options

The options regarding search functionalities itself are rather brief, its defaults are summarised in this snippet:

```js
initMorsels({
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
        // Whether to cache **all** field stores on initialisation
        // defaults to the same setting in the indexer configuration file
        cacheAllFieldStores: undefined,

        // Any index file >= this size requested before will be persistently cached
        // This option does not affect field stores.
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

This controls the maximum number of terms to search for manual wildcard [suffix searches](./search_features.md#wildcard-search).

#### Term Proximity Ranking

`useQueryTermProximity = true`

If positions are indexed, document scores are also scaled by how close query expressions or terms are to each other. This boosts result relevance significantly.

#### Caching Options (Advanced)

This is discussed more in the chapter on [larger collections](./indexer/larger_collections.md).
