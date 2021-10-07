# Search Configuration

All search time related options can be provided through the `initMorsels` function, exposed by `@morsels/search-ui`.

## Forenotes

### Fullscreen Pop-Up and Dropdown UI Versions

The search UI provides 2 different interfaces, a dropdown and fullscreen / "popup" version.

The dropdown version is used for desktop devices by default, which is attached to the element specified by `inputId`.
For mobile devices, the fullscreen version is used instead.

### Default Rendering Output / Purpose

It is also worth rehashing that the default result generation assumes the simple but common use case of **linking to a source document** (`<a />` tag). 

This is achieved through an internal `_relative_fp` field that is stored during indexing, and retrieved during search time. The combination of the base url from which to retrieve these source files (see `sourceFilesUrl` below) and this `_relative_fp` field forms the full source document link, which is used for:
- Attaching a link to the source document in the generated result match
- Retrieving the source document

Therefore, source documents are assumed to be available. You may refer to the section on [generating](#generating-result-previews) result previews otherwise.

## Summary

The available options and their format in this page are detailed in the following code block. Options which warrant more explanation are also elaborated in the subsequent sections of this page.

```js
initMorsels({
    // Options belonging to @morsels/search-lib, the search library package
    searcherOptions: {
        // Base url of output directory from cli tool
        url: 'http://192.168.10.132:3000/output/',
        
        // Whether to use substring query term expansion
        numberOfExpandedTerms: 3,
        
        // Override for using query term proximity ranking or not.
        // Disabled for mobile devices by default
        useQueryTermProximity: true,
    },
    
    // Options for @morsels/search-ui, the user interface package
    
    // Id of input element to attach the search dropdown to
    inputId: 'morsels-search',

    // Debounce time for the input handlers
    inputDebounce: isMobile ? 275 : 200,
    
    // Mandatory, by default - base url for sourcing .html / .json files
    sourceFilesUrl: 'http://192.168.10.132:3000/source/',
    
    // Customise search result outputs, UI behaviour
    render: {
        enablePortal?: boolean | 'auto',
        portalTo?: HTMLElement,
        opts: {
            dropdownAlignment: 'left' | 'right'
        },
        // refer to Renderers section for the other options
    }
});
```

## Basic User Interface Behaviours

This section details the available options, necessary inputs and some utility methods controlling the some user interface behaviour.

The subsequent section on [renderers](./search_configuration_renderers.md) provides a more advanced API (that really shouldn't be needed) to customise the html output. If you have a configuration use case that cannot be achieved without these APIs, and you think should be included as a simpler configuration option here, feel free to raise a feature request on the Github repository.

```ts
{
    inputId?: string,
    render: {
        enablePortal?: boolean | 'auto',
        portalTo?: HTMLElement,
        // ...
    }
}
```

**`inputId`**

This option tells morsels which input element to use for the **dropdown version** of its search ui.

If this is unspecified, the `show / hide` APIs below must be used to bring up the **fullscreen UI**.


**`enablePortal = 'auto'`**

This parameter tells morsels whether to use the fullscreen search UI for mobile devices when the original `<input>` specified by `inputId` is focused.

The default value of `'auto'` configures this according to mobile device detection (`true` if it is a mobile device), and also adds a simple window resize handler to automatically hide the corresponding UI if the window is resized.

You can set this to `true` / `false` to always prefer the fullscreen or dropdown version instead when the original `<input>` is focused.

If `inputId` is unspecified, this option will not do anything.

**`portalTo = document.getElementsByTagName('body')[0]`**

This parameter tells morsels which element to attach the fullscreen search UI to, which uses `fixed` positioning.

**`show() / hide()`**

```ts
const { show, hide } = initMorsels(/* ... */);
```

The default behaviour of showing the fullscreen search UI on focusing the input may be insufficient in some cases, for example showing the UI when clicking a search icon.

You may call the `show()` function returned by the initMorsels call in such a case for manual control. Correspondingly, the `hide()` method hides the fullscreen interface, although, this shouldn't be needed since there's a close button is available by default.


## Automatic Term Expansion

(`numberOfExpandedTerms`)

By default, stemming is turned off in the [language modules](indexing_configuration.md#language). This does mean a bigger dictionary (but not that much anyway), and lower recall, but much more precise searches.

To provide a compromise for recall, query terms that are similar to the searched term are added to the query, although with a lower weight.

For both of the [language](./indexing_configuration.md#latin-tokenizer) modules available currently, this is only applied for the last query term, and if the query string does not end with a whitespace. An implicit wildcard (suffix) search is performed on this term. (quite similar to Algolia Docsearch's behaviour)

## Term Proximity Ranking

(`useQueryTermProximity`)

Document scores are also scaled by how close query expressions or terms are to each other, if positions are indexed.
This may be costly for mobile devices however, and is disabled by default in such cases.

## Generating Result Previews

There are 3 ways to generate result previews, the first of the below being the default.

Unless you have modified the default result renderer (covered in the renderers page), morsels requires **at least** one of the `body` / `heading` / `title` fields (covered in the next section on indexing configuration).

### 1. From Source Documents (default)

(`sourceFilesUrl`)

When the below option (field stores) is not configured or unavailable, morsels will attempt to fetch the source document from `sourceFilesUrl` adjoined with the relative file path of the document at the time of indexing. The source document is then reparsed, and its fields are extracted again in order to generate result previews.

Note that this option is only applicable for indexed html and json files at this time.
As csv files are often used to hold multiple documents (and can therefore get very large), it is unsuitable to be used as a source for search result previews. In this case, the subsequent section details the alternative method of generating result previews.

### 2. From Field Stores

If source documents are unavailable, morsels is able to generate result previews from its json field stores generated at indexing time.

In order to specify what fields to store, please take a look at the `do_store` option in this [section](./indexing_configuration.md#fields_config) of the indexing configuration page. To use this method of result preview generation for the default use case / result rendering behaviour, simply enable the `do_store` option for the `heading`, `body`, and `title` fields.

You may also wish to use this method even if source documents are available, if filesystem bloat isn't too much of a concern. Apart from avoiding the additional http requests, the internal json field store comes packed in a format that is more performant for search-ui to perform result preview generation on.

### 3. Alternative Rendering Outputs (advanced)

It is also possible to create your own result [renderer](./search_configuration_renderers.md) to, for example:
- attach an event handler to call a function when a user clicks the result preview
- retrieve and generate result previews from some other API.

Nevertheless, the section ["From Field Stores"](#from-field-stores) above would still be relevant as it provides the basis for retrieving a document's fields (e.g. a document id with which to call an API).

## Mobile Device Detection

It may helpful to note that mobile devices are detected through a simple `window.matchMedia('only screen and (max-width: 1024px)').matches` query at initialisation time, which may not be accurate.

This is used for determining some default settings, namely:
- Query term proximity ranking is disabled
- The input debounce time, which is slightly longer for mobile devices
- Whether to use the fullscreen "portal"-ed version of the user interface instead
 
Overrides may be provided through the options above if this detection method is inadequate.
