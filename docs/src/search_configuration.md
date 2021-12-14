# Search Configuration

All search time related options can be provided through the `initMorsels` function, exposed by `@morsels/search-ui`.

There are 2 categories of options, the first being related to the search library (`@morsels/search-lib`), and the second the user interface (`@morsels/search-ui`).

## Forenote on Mobile Device Detection

Several options in both the search library and UI are by default tuned based on whether the client is a mobile device.

Some examples of tuned settings for mobile devices:
- Query term proximity ranking is disabled
- Whether to use the fullscreen version (more later) of the user interface instead

The check is done through a simple `window.matchMedia('only screen and (max-width: 1024px)').matches` query at initialisation time, which may not be robust enough for your use case.
 
An override may be provided through the `isMobileDevice` option shown below, which is simply a function returning a boolean.

```ts
initMorsels({
    isMobileDevice: () => true,
})
```

---

## Search UI Options

Search UI options are organised under the `uiOptions` key:

```ts
initMorsels({
    uiOptions: {
        // ... options go here ...
    }
})
```

For brevity, this page covers only a subset of the most important options.

The subsequent section on [renderers](./search_configuration_renderers.md) provides a more advanced API to customise the html output. If you have a configuration use case that cannot be achieved without these APIs, and you think should be included as a simpler configuration option here, feel free to raise a feature request!

### Input Element


| Option      | Default Value | Description |
| ----------- | ----------- | ----------- |
| `input`     | `'morsels-search'` | id of the input element or a html element reference |
| `inputDebounce`     | `100` | debounce time of keystrokes to the input element |
| `preprocessQuery`   | `(q) => q` | any function for preprocessing the query. Can be used to add a [field filter](./search_features.md#field-search) for example. |

The `input` option is the most important option, and required in most use cases. Its purpose varies depending on the ui mode specified below.

### UI Mode

`mode = 'auto'`

The search UI provides 4 main different behaviours.

To try the different modes out, head on over to the [mdbook plugin](./getting_started_mdbook.md#preview) page, which provides various buttons for switching the modes in this documentation.

| Mode        | Description |
| ----------- | ----------- |
| `"auto"`        | This option is the **default**, and combines the `dropdown` and `fullscreen` options below. If a mobile device is detected as per the [earlier section](#forenote-on-mobile-device-detection), the `fullscreen` mode is used. Otherwise, the `dropdown` mode is used.<br><br>A debounced window resize handler is also attached that reruns the mobile device check whenever the window is resized.   |
| `"dropdown"`    | This wraps the specified `input` element with a root container. Search results are displayed using an additional container attached to this root container.    |
| `"fullscreen"`  | This option creates a completely distinct root container with its own input element, and attaches it to the `<body>` element. Under the default stylesheet, the user interface is fullscreen for devices satisfying `max-width: 1025px`, and takes up roughly 50% - 75% of the screen otherwise.<br><br>If the `input` element is specified, the root container is attached whenever the `input` is focused.<br><br>Alternatively, one may use the `showFullscreen` and `hideFullscreen` functions returned by the `initMorsels` function to toggle the UI. This is also the only use case where you would not need to specify the `input` element.    |
| `"target"`      | This option is the most flexible, and is used by the mdbook plugin and this documentation by default. The `input` element must be specified, but only for attaching keystroke listeners. No dom manipulation is performed unlike the `dropdown` or `auto` modes.<br><br>The search results are output to a custom target element of choice.    |


#### UI Mode Specific Options

There are also several options specific to each UI. Note that `dropdown` and `fullscreen` options are both applicable to the `auto` mode.

| Mode        | Option                | Default          | Description |
| ----------- | -----------           | -----------      | ----------- |
| `dropdown`  | `dropdownAlignment`   | `'bottom-end'`        | `'bottom-start'` or `'bottom-end'`. Which side of the input element to align the dropdown results container and dropdown seperator against. The alignment of the dropdown container will be automatically flipped horizontally to ensure the most optimal size (see [floating-ui](https://floating-ui.com/docs/size#using-with-flip) 's docs for a demonstration).
| `fullscreen`| `fullscreenContainer` | `<body>` element | Id of the element, or an element reference to attach the separate root container to.
| `target`    | `target`              | -                | Id of the element, or an element reference to attach results to. Required if using `mode='target'`.

#### Manually Showing / Hiding the Fullscreen UI

```ts
const { showFullscreen, hideFullscreen } = initMorsels(/* ... */);
```

The default behaviour of showing the fullscreen search UI when focusing the input may be insufficient, for example to show the UI when clicking a "search icon".

You may call the `showFullscreen()` function returned by the initMorsels call in such a case for manual control. Correspondingly, the `hideFullscreen()` method hides the fullscreen interface, although, this shouldn't be needed since a close button (or by pressing `esc`) is available by default.

These functions can also be used under `mode='auto'` if desired.


### Options for Generating Result Previews

There are 3 ways to generate result previews, the first of the below being the default.

Unless you have modified the default result renderer (covered in the next page on renderers), morsels requires **at least** one of the `body` / `heading` / `title` fields. This is configured by default, and covered in the next section on indexing configuration in more detail.


### Default Rendering Output / Purpose

The default result generation assumes the simple but common use case of **linking to a source document** (`<a />` tag). 

Therefore, source documents are assumed to be available. To generate alternative outputs (e.g. buttons, perform some action), you will need to use option 3 below.

#### 1. From Source Documents (default)

`sourceFilesUrl`

When option 2 below (field stores) is not configured or unavailable, morsels will attempt to fetch the source document from **`sourceFilesUrl` adjoined with the relative file path of the document** at the time of indexing. The source document is then reparsed, and its fields are extracted again in order to generate result previews.

The `_relative_fp` field is an internally generated field that can be stored during indexing, and retrieved during search time. The combination of the base url from which to retrieve these source files (`sourceFilesUrl`) and this field forms the full source document link, used for:
- Attaching a link to the source document in the generated result match
- Retrieving the source document

Note that this option is only applicable for indexed html and json files at this time.

As csv files are often used to hold multiple documents (and can therefore get very large), it is unsuitable to be used as a source for search result previews. In this case, options 2 or 3 can be used.

#### 2. From Field Stores

If source documents are unavailable, morsels is able to generate result previews from its own json field stores generated at indexing time.

In order to specify what fields to store, take a look at the `do_store` option in this [section](./indexing_configuration.md#fields_config) of the indexing configuration page. To use this method of result preview generation for the default use case / result rendering behaviour, enable the `do_store` option for the relevant fields.

You may also wish to use this method even if source documents are available, if filesystem bloat isn't too much of a concern. Apart from avoiding the additional http requests, the internal json field store comes packed in a format that is more performant to perform result preview generation on.

#### 3. Alternative Rendering Outputs (advanced)

It is also possible to create your own result [renderer](./search_configuration_renderers.md) to, for example:
- attach an event handler to call a function when a user clicks the result preview
- retrieve and generate result previews from some other API.

Nevertheless, the section ["From Field Stores"](#from-field-stores) above would still be relevant as it provides the basis for retrieving a document's fields (e.g. a document id with which to call an API).

This is covered in more detail in the next page.

### Results Per Page

`resultsPerPage = 8`

In all UI modes, an infinite scrolling intersection observer is attached to the last search result, if any. When triggered, search result previews are fetched and/or generated for a number of these results only.

Lowering this can have a noticeable performance improvement on result generation, as more `.html / .json` files have to be retrieved on-the-fly, parsed, and processed. This is especially true if using option 1 above.

---

## Search Library Options

The options for the search library are rather brief, and can be summarised in this snippet:

```js
initMorsels({
    // Options belonging to @morsels/search-lib, the search library package
    searcherOptions: {
        // Base url of output directory that the cli tool generated
        url: 'http://192.168.10.132:3000/output/',
        
        // Maximum number of terms for query term expansion
        numberOfExpandedTerms: 3,
        
        // Override for using query term proximity ranking or not.
        // Disabled for mobile devices by default
        useQueryTermProximity: true,

        // Whether to retrieve all field stores on initialisation
        // (see chapter "Tradeoffs" for more details)
        cacheAllFieldStores: true,
    },
});
```

### Automatic Term Expansion

`numberOfExpandedTerms`

By default, stemming is turned off in the [language modules](indexing_configuration.md#language). This does mean a bigger dictionary (but not that much anyway), and lower recall, but much more precise searches.

To provide a compromise for recall, query terms that are similar to the searched term are added to the query, although with a lower weight.

For both of the [language](./indexing_configuration.md#latin-tokenizer) modules available currently, this is only applied for the last query term, and if the query string does not end with a whitespace. An implicit wildcard (suffix) search is performed on this term. (quite similar to Algolia Docsearch's behaviour)

### Term Proximity Ranking

`useQueryTermProximity`

If positions are indexed, document scores are also scaled by how close query expressions or terms are to each other.
This may be costly for mobile devices however, and is disabled by default in such cases.
