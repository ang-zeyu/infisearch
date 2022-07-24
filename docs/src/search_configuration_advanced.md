# Search UI

This section covers some advanced options for changing **how** (this page) result previews are generated or **what** (next page) they display.

You should not need to alter this unless:
- **How:** Your collection is fairly large (e.g. `> 200MB`), you want to use source files to generate previews instead to reduce file bloat from fragmenting field stores.
- **What:** You want to change what data is displayed in the result previews, or even what happens (e.g. attaching an event handler).

You should have at least read the chapter on fields, on the [default field configurations](./indexer/fields.md#default-field-configuration) in particular.

## Options for Generating Result Previews

There are 3 available ways of generating result previews, and some [presets](./indexer/larger_collections.md) are available to help with switching between options 1 and 2 easily. How these presets are configured individually are also covered under [larger collections](./indexer/larger_collections.md), as the rationale for using alternative methods links closely to your collection size. (except option 3)

#### 1. From Source Documents

When the required fields (`heading` / `body` / `title`) are not found in Morsels' field stores, an attempt will be made to retrieve and reparse the source document and its fields in order to generate result previews.

Note that this option is only applicable for indexed HTML and JSON files at this time.

In addition, this option will not work with the [`_add_files` field](./indexer/indexing.md#indexing-multiple-files-under-one-document) which allows indexing multiple files under a single document.

#### 2. From Field Stores (default)

Morsels is also able to generate result previews from its JSON field stores [saved](./indexer/fields.md#storing-fields-do_store) during indexing.

You may also wish to use this method even if source documents are available, if filesystem bloat isn't too much of a concern. Apart from avoiding the additional http requests, the internal json field store comes packed in a format that is more performant to perform result preview generation on.

#### 3. Alternative Rendering Outputs (advanced)

It is also possible to create your own result [renderer](./search_configuration_renderers.md) to, for example:
- attach an event handler to call a function when a user clicks the result preview
- retrieve and generate result previews from some other API.

Nevertheless, the section ["From Field Stores"](#from-field-stores) above would still be relevant as it provides the basis for retrieving a document's fields (e.g. a document id with which to call an API).

This is covered in more detail under [Altering HTML Outputs](./search_configuration_renderers.md) in the next page.
