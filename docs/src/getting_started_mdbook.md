# mdbook-morsels

`mdbook-morsels` is a simple search plugin replacement for [mdBook](https://github.com/rust-lang/mdBook) to use morsels' search interface and library instead of elasticlunr.js.

## What, why?

MdBook already has its own built-in search function utilising elasticlunr, which works well enough for most cases. This plugin was mainly created as:
1. A proof-of-concept to integrate morsels with other static site generators easily
2. A personal means to set up document deployment workflows in CI scripts

You may nonetheless want to use this plugin if you need Morsels' extra features. Some examples:
- you require PDF file support, or JSON file support to link to out-of-domain pages.
- spelling correction, automatic prefix search, term proximity ranking, etc.
- you prefer the look of the UI here.

## Styling

This plugin uses the css variables provided by the 5 main default themes in mdBook to style the search user interface. Switch the themes in this documentation to try out the different themes!

**Note:** The "Morsels" theme is not included in the plugin and is specific to this documentation. It is included only to show the default styling (without this plugin).

## Installation

Install the executable either using `cargo install mdbook-morsels`, or download and add the [binaries](https://github.com/ang-zeyu/morsels/releases) to your `PATH` manually.

Then, minimally add the first two configuration sections below to your `book.toml` configuration file:

```toml
[output.html.search]
enable = false               # disable the default mdBook search feature implementation

[preprocessor.morsels]
command = "mdbook-morsels"

# Plugin configuration options (optional)
[output.morsels]
# See search configuration page, or use the buttons below
mode = "target"

# Relative path to a Morsels indexer configuration file from the project directory.
# The config file will also automatically be created here if it dosen't exist.
config = "morsels_config.json"
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
