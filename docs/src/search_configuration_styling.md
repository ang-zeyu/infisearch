# Styling

## Themes

Morsels provides 2 built-in themes by default, which correspond to the 2 stylesheets in the [releases](https://github.com/ang-zeyu/morsels/releases).

These 2 stylesheets also expose a wide range of css variables which you can alter as needed.

Head on over to the demo site [here](https://morsels-search.com) to try them out!

### Light

#### CDN link

```html
<!-- Replace "v0.2.5" as appropriate -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/ang-zeyu/morsels@v0.2.5/packages/search-ui/dist/search-ui-light.css" />
```

#### Preview

<style>
.image-container {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: center;
}

@media screen and (max-width: 1129px) {
    .image-container > img:first-child {
        margin-bottom: 10px;
        min-width: 300px;
        max-width: 500px;
    }

    .image-container > img:last-child {
        width: 300px;
    }
}

@media screen and (min-width: 1130px) {
    .image-container > img:first-child {
        margin-right: 10px;
        height: 440px;
    }

    .image-container > img:last-child {
        height: 440px;
    }
}
</style>

<div class="image-container">
<img src="./images/light-theme.png" alt="Preview of light theme">
<img src="./images/light-theme-fullscreen.png" alt="Preview of light theme (fullscreen)">
</div>

### Dark

#### CDN link

```html
<!-- Replace "v0.2.5" as appropriate -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/ang-zeyu/morsels@v0.2.5/packages/search-ui/dist/search-ui-dark.css" />
```

#### Preview

<div class="image-container">
<img src="./images/dark-theme.png" alt="Preview of dark theme">
<img src="./images/dark-theme-fullscreen.png" alt="Preview of dark theme (fullscreen)">
</div>

## Input Element As a Button

Where the `input` option passed to `initMorsels` is concerned, Morsels adopts a minimally invasive approach to styling, preferring to leave this to your individual site's preferences.

For reasons of accessbility however, some minimal styling is applied when using the [fullscreen UI](./search_configuration.md#ui-mode) to convey the intention of a button. This is limited to:
- A `background` + `box-shadow` + `color` application on *focus* only

  These are applied with a `!important` modifier as they are key to conveying keyboard focus, but are also overridable easily with Morsels' css variables.
- `cursor: pointer` application on *hover* only

You may override and addon to these styles as needed, to convey the intention of a button further.

If using the default [UI mode](./search_configuration.md#ui-mode) of auto, you can also set a different [placeholder](./search_configuration.md#ui-mode-specific-options), and use the `.morsels-button-input` selector to apply your styles only when the fullscreen UI is in use. For example,

```css
.morsels-button-input:focus:not(:hover) {
    background: #6c757d !important;
}
```

Accessibility labels and roles are automatically set however, so you needn't worry about those.
