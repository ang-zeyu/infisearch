# Styling

<style>
.image-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
}

.image-container > img {
    margin-top: 10px;
    width: 500px;
    max-width: 100%;
}
</style>

## Themes

InfiSearch provides 3 built-in themes by default, which correspond to the 3 stylesheets in the [releases](https://github.com/ang-zeyu/infisearch/releases).

These 3 stylesheets also expose a wide range of css variables which you can alter as needed.

Head on over to the demo site [here](https://infi-search.com) to try them out!


### Light

#### CDN link

```html
<!-- Replace "v0.10.0" as appropriate -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/ang-zeyu/infisearch@v0.10.0/packages/search-ui/dist/search-ui-light.css" />
```

#### Preview

<div class="image-container">
<img src="./images/light-theme.png" alt="Preview of light theme">
<img src="./images/light-theme-dropdown.png" alt="Preview of light theme (dropdown)">
</div>


### Basic

#### CDN link

```html
<!-- Replace "v0.10.0" as appropriate -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/ang-zeyu/infisearch@v0.10.0/packages/search-ui/dist/search-ui-basic.css" />
```

<div class="image-container">
<img src="./images/basic-theme-dropdown.png" alt="Preview of basic theme (dropdown)">
</div>

### Dark

#### CDN link

```html
<!-- Replace "v0.10.0" as appropriate -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/ang-zeyu/infisearch@v0.10.0/packages/search-ui/dist/search-ui-dark.css" />
```

#### Preview

<div class="image-container">
<img src="./images/dark-theme-dropdown.png" alt="Preview of dark theme (dropdown)">
</div>

## Styling the Fullscreen UI Input Button

InfiSearch is minimally invasive in styling your `<input>` element (except for the one that comes with the fullscreen UI), leaving this to your site's design.

Notably however, for accessibility, some minimal styling is applied when using the [fullscreen UI](./search_configuration.md#ui-mode) to convey the intention of a button (which opens the fullscreen UI). This is limited to:
- A `background` & `box-shadow` & `color` application on *focus*

  These are applied with a `!important` modifier as they are key to conveying keyboard focus, but are also overridable easily with InfiSearch's css variables.
- `cursor: pointer` application on *hover*

**Applying Input Button Styles under `mode='auto'`**

If using the default [UI mode](./search_configuration.md#ui-mode) of `auto`, which switches between the dropdown and fullscreen UI dynamically, you can also set a different [placeholder](./search_configuration.md#ui-mode-specific-options), and/or use the `.infi-button-input` selector to apply your styles only if the fullscreen UI is used. For example,

```css
.infi-button-input:focus:not(:hover) {
    background: #6c757d !important;
}
```
