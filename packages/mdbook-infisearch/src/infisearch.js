infisearch.init({
  searcherOptions: {
    url: base_url + 'infisearch_output/',
  },
  uiOptions: {
    mode,
    dropdownAlignment: 'bottom-start',
    target: document.getElementById('infisearch-mdbook-target'),
    fsButtonPlaceholder: 'Search',
    sourceFilesUrl: base_url,
    resultsRenderOpts: {
      searchedTermsParam: 'search',
    },
    multiSelectFilters: [
      { fieldName: 'partTitle', displayName: 'Section', defaultOptName: 'None' },
    ],
  },
});

document.getElementById('infi-search').addEventListener('keydown', (ev) => {
  if (['ArrowLeft', 'ArrowRight'].includes(ev.key)) {
    ev.stopPropagation(); // used in global listener to change pages
    return;
  }
});

if (window.location.search) {
  // Adapted from the original searcher.js for mdbook
  // https://github.com/rust-lang/mdBook/blob/master/src/theme/searcher/searcher.js
  const target = document.getElementById('content');
  const marker = new Mark(target);

  function doSearchOrMarkFromUrl() {
    // Check current URL for search request
    var url = new URL(window.location.href);
    var urlParams = new URLSearchParams(url.search);

    if (urlParams.has('search')) {
      var words = JSON.parse(decodeURIComponent(urlParams.get('search')));
      marker.mark(words);

      var markers = document.querySelectorAll('mark');
      function hide() {
        for (var i = 0; i < markers.length; i++) {
          markers[i].classList.add('fade-out');
          window.setTimeout(function () { marker.unmark(); }, 300);
        }
      }
      for (var i = 0; i < markers.length; i++) {
        markers[i].addEventListener('click', hide);
      }
    }
  }
  doSearchOrMarkFromUrl();
}