# mdbook-infisearch

`mdbook-infisearch` is a simple search plugin replacement for [mdBook](https://github.com/rust-lang/mdBook) to use InfiSearch's search interface and library instead of elasticlunr.js.

## What, why?

MdBook already has its own built-in search function utilising elasticlunr, which works well enough for most cases. This plugin was mainly created as:
1. A proof-of-concept to integrate InfiSearch with other static site generators easily
2. A personal means to set up document deployment workflows in CI scripts

You may nonetheless want to use this plugin if you need InfiSearch's extra features. Some examples:
- you require PDF file support, or JSON file support to link to out-of-domain pages.
- spelling correction, automatic prefix search, term proximity ranking, etc.

## Styling

This plugin uses the css variables provided by the 5 main default themes in mdBook to style the search user interface. Switch the themes in this documentation to try out the different themes!

**Note:** The default InfiSearch theme is not included in the plugin. To see the default styling, head on over to the [styling](./search_configuration_styling.md) page or view the [demo](https://ang-zeyu.github.io/infisearch-website) site.

## Installation

Install the executable either using `cargo install mdbook-infisearch`, or download and add the [binaries](https://github.com/ang-zeyu/infisearch/releases) to your `PATH` manually.

Then, minimally add the first two configuration sections below to your `book.toml` configuration file:

```toml
[output.html.search]
# disable the default mdBook search feature implementation
enable = false

[preprocessor.infisearch]
command = "mdbook-infisearch"

[output.infisearch]  # this header should be added
# Plugin configuration options (optional)
# See search configuration page, or use the buttons below
mode = "target"

# Relative path to a InfiSearch indexer configuration file from the project directory.
#
# If you are creating this for the first time, let this point to a non-existent file
# and the config file will be created with Infisearch's settings tailored for mdBook.
config = "infi_search.json"
```

## Preview

Use the following (non-canonical, documentation specific) buttons to try out the different [**`mode`** parameters](search_configuration.md#ui-mode).

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

You can also try out the different **themes** on this documentation using mdBook's paintbrush icon!
