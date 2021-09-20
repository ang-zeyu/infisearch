/*
 Morsels documentation specific.

 For showcasing the default morsels theme (without mdbook)
 */

(function () {
    // Add a new theme: https://github.com/rust-lang/mdBook/issues/605#issuecomment-362927102
    const theme_list = document.getElementById('theme-list');
    const theme = document.createElement('li');
    theme.innerHTML = '<button class="theme" id="morsels-theme">Morsels</button>';
    theme_list.appendChild(theme);
})()
