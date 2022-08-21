# Search Configuration

All search related options can be provided through the `initMorsels` function, exposed by the search bundle.

There are 2 categories of options, the first related to the user interface, and the latter search functionalities.

## Search UI Options

Search UI options are organised under the `uiOptions` key:

```ts
initMorsels({
    uiOptions: {
        // ... options go here ...
    }
})
```

#### Input Element

| Option      | Default Value | Description |
| ----------- | ----------- | ----------- |
| `input`     | `'morsels-search'` | `id` of the input element or a HTML element reference |
| `inputDebounce`     | `100` | debounce time of keystrokes to the input element |
| `preprocessQuery`   | `(q) => q` | any function for preprocessing the query. Can be used to add a [field filter](./search_features.md#field-search) for example. |

The `input` element is the most important option, and is required in most cases. Its purpose varies depending on the `mode` specified below.

#### UI Mode

The search UI provides 4 main different behaviours.

To try the different modes out, head on over to the [mdbook plugin](./getting_started_mdbook.md#preview) page, which provides various buttons for switching the modes in this documentation.


| Mode        | Details |
| ----------- | ----------- |
| `"auto"`        | This option is the **default**, and combines the `dropdown` and `fullscreen` options below. If a mobile device is [detected](#changing-the-mobile-device-detection-method), the `fullscreen` mode is used. Otherwise, the `dropdown` mode is used.<br><br>An event handler is also attached that reruns this adjustment whenever the window is resized.   |
| `"dropdown"`    | This wraps the specified `input` element with a root container. Search results are placed in a `<ul>` container next to the input element.    |
| `"fullscreen"`  | This option creates a completely distinct root container (complete with its own input, backdrop, close button), and attaches it to the `<body>` element.<br><br>If the `input` element is specified, a click handler is attached to open this UI. For keyboard accessibility, some minimal, but overidable [styling](./search_configuration_styling.md#input-element) is applied to the input element.<br><br>This UI can also be shown/hidden [programatically](#manually-showing--hiding-the-fullscreen-ui), which is also *the only case* you would not need to specify the `input` element.    |
| `"target"`      | This option is the most flexible, and is used by the mdbook plugin (by default) and this documentation. The `input` element must be specified, where keystroke event listeners are attached. No dom manipulation is performed unlike the `dropdown` or `auto` modes.<br><br>The search results are output to a custom `target` element of choice.    |

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
| `dropdown`  | `dropdownAlignment`   | `'bottom-end'`          | `'bottom'` or `'bottom-start'` or `'bottom-end'`.<br><br>This is the side of the input element to align the dropdown results container and dropdown seperator against.<br><br>The alignment will also be automatically flipped horizontally to ensure the most optimal placement.
| `auto`      | `fsInputButtonText`        | `undefined`| Placeholder override for the `input` if the fullscreen UI is in use.<br><br>This is added for keyboard [accessibility](./search_configuration_styling.md#input-element-as-a-button).
| `fullscreen` | `fsInputLabel`        | `'Search'` | Accessibility label for the original input element, when the fullscreen UI is in use.
| `fullscreen` | `fsContainer`         | `<body>` element        | `id` of the element, or an element reference to attach the separate root container to.
| `fullscreen` | `fsPlaceholder`       | `'Search this site...'` | Placeholder of the input element in the fullscreen UI.
| all except `target`         | `tip`                 | `true`        | Whether to show the tip icon. When hovered over, this shows advanced usage information (e.g. how to perform phrase queries).
| `target`    | `target`              | `undefined`                       | `id` of the element, or an element reference to attach results to.<br><br>Required if using `mode='target'`.

#### General Options

| Option                | Default                 | Description |
| -----------           | -----------             | ----------- |
| `label`               | `'Search this site'`    | Accessibility label for the fullscreen UI input.
| `resultsLabel`        | `'Site results'`        | Accessibility label for result `listbox`es.
| `maxSubMatches`       | `2`                     | Maximum number of heading-body pairs to show for a document.

#### Manually Showing / Hiding the Fullscreen UI

```ts
const { showFullscreen, hideFullscreen } = initMorsels({ ... });
```

You may call the `showFullscreen()` function returned by the initMorsels call to programatically show the fullscreen search UI.

Correspondingly, the `hideFullscreen()` method hides the fullscreen interface, although, this shouldn't be needed since a close button is available by default (the <kbd>Esc</kbd> key works too).

These methods can also be used under `mode="auto"`.

#### Results Per Page

`resultsPerPage = 8`

In all UI modes, an infinite scrolling intersection observer is attached to the last search result. When triggered, search result previews are fetched and generated for the next `resultsPerPage` number of results.

#### Changing The Mobile Device Detection Method


If the client is a "mobile device", the fullscreen version of the user interface is used for `mode='auto'`.

This check is done through a simple media query, which may not be adequate for your use case.

```js
window.matchMedia('only screen and (max-width: 1024px)').matches
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
        // Base url of output directory that the cli tool generated
        url: 'http://192.168.10.132:3000/output/',
        
        // Maximum number of terms for query term expansion (see below for more info)
        numberOfExpandedTerms: 3,
        
        // Override for using query term proximity ranking or not. (see below for more info)
        useQueryTermProximity: true,

        // ---------------------------------------------------------------
        // Caching Options
        // Whether to cache **all** field stores on initialisation
        // defaults to the same setting in the indexer configuration file
        cacheAllFieldStores: undefined,

        // Any index file >= this size requested before will be persistently cached
        // This option does not affect field stores.
        plLazyCacheThreshold: 0,
        // ---------------------------------------------------------------

        // The maximum number of results to retrieve (unlimited if null).
        resultLimit: null,
    },
});
```

#### Automatic Term Expansion

`numberOfExpandedTerms = 3`

Stemming is turned off by [default](./language.md#ascii-tokenizer). This does mean a bigger dictionary (but not too much usually), and lower recall, but much more precise searches.

To provide a compromise for recall, query terms that are similar to the searched term are added to the query, although with a lower weight.

For all [language modules](./language.md) available currently, this is only applied for the last query term, and if the query string does not end with a whitespace. An implicit wildcard (suffix) search is performed on this term.

#### Term Proximity Ranking

`useQueryTermProximity = true`

If positions are indexed, document scores are also scaled by how close query expressions or terms are to each other. This is the more time consuming part of Morsels' document ranking algorithm, but still incredibly fast.

#### Caching Options (Advanced)

This is discussed more in the chapter on [larger collections](./indexer/larger_collections.md).
