# mdbook-morsels

`mdbook-morsels` is a simple search plugin replacement for [mdbook](https://github.com/rust-lang/mdBook) to use morsels' search interface and library instead of elasticlunr.js.

It uses the css variables provided by the 5 main default themes in mdbook to style the search user interface. You can switch the themes in this documentation to try out the different themes.

Note: The "Morsels" theme is not included in the plugin and is specific to this documentation. It is included only to show the default styling (without mdbook-morsels).

## Installation

You will need to have installed the following command-line crates (`cargo install <crate-name>` or with the binaries in your `PATH`):
- mdbook
- mdbook-morsels
- morsels_indexer

Then, minimally add the first two configuration sections below to your `book.toml` configuration file:

```toml
[output.html.search]
enable = false               # disable the default mdbook search feature implementation

[preprocessor.morsels]
command = "mdbook-morsels"
renderer = ["html"]          # this should only be run for the html renderer

# Plugin configuration options (optional)
[output.morsels]

# See search configuration page
mode = "target"

# Relative path to the indexer configuration file from the root project directory
# This will automatically be created if it dosen't exist.
config = "_morsels_config.json"

# Don't add the default stylesheet from morsels/search-ui,
# nor the inline css variables for the default mdbook themes
no-css = false
```

## Preview

Try out the different **themes** on this documentation using mdbook's paintbrush icon!

You may also use the following (non-canonical, documentation specific) buttons to try out the different [**`mode`** parameters](search_configuration.md#ui-mode).

![mdbook morsels plugin preview](./images/mdbook-preview.gif)
*Gif of `mode='fullscreen'` across the different themes*

Also note that unlike the default search feature, the search bar is always there -- there is no search icon on the navbar to click. I am still trying to figure how to add this nicely (without runtime hacks) within mdbook's plugin framework :)
