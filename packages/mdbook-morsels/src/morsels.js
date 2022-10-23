morsels.initMorsels({
  searcherOptions: {
    url: base_url + 'morsels_output/',
  },
  uiOptions: {
    mode,
    dropdownAlignment: 'bottom-start',
    target: document.getElementById('morsels-mdbook-target'),
    sourceFilesUrl: base_url,
    resultsRenderOpts: {
      addSearchedTerms: 'search',
    },
  },
});

document.getElementById('morsels-search').addEventListener('keydown', (ev) => {
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