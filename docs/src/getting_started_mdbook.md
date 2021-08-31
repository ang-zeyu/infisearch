# mdbook-morsels

`mdbook-morsels` is a simple proof-of-concept search plugin for [mdbook](https://github.com/rust-lang/mdBook) to use morsels' search functions instead.

It uses the css variables provided by the 5 main default themes in mdbook to style the search user interface. You can switch the themes in this documentation to try out the different themes.

## Installation

You will need to have installed the following command-line crates (`cargo install <crate-name>`):
- mdbook
- mdbook-morsels
- morsels_indexer

Then add the first two configuration sections below to your `book.toml` configuration file:

```toml
[output.html.search]
enable = false               # disable the default mdbook search feature implementation

[preprocessor.morsels]
command = "mdbook-morsels"
renderer = ["html"]          # this should only be run for the html renderer

# Plugin configuration options (optional)
[output.morsels]

# Force morsels to use the fullscreen popup UI version,
# instead of dynamically switching between the dropdown / popup version for desktop / mobile devices
portal = true

# Relative path to the indexer configuration file from the root project directory
# This will automatically be created if it dosen't exist.
config = "_morsels_config.json"

# Don't add the default stylesheet from morsels/search-ui,
# nor the inline css variables for the default mdbook themes
no-css = false
```

## Preview

Here's what it looks like across the five different themes currently.

Also note that unlike the default search feature, the search bar is always there! There is no search icon on the navbar to click. I am still trying to figure how to add this nicely within mdbook's plugin framework :).

![mdbook morsels plugin preview](./images/mdbook-preview.gif)
