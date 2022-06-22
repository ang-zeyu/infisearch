export async function fetchPl(plName, numPlsPerDir, baseUrl) {
  const plUrl = `${baseUrl}pl_${Math.floor(plName / numPlsPerDir)}/pl_${plName}.json`;
  const cacheName = `morsels:${baseUrl}`;

  function fetchUrl(url) {
    return fetch(url).then((resp) => resp.arrayBuffer());
  }

  try {
    const cache = await caches.open(cacheName);
    const cacheResp = await cache.match(plUrl);
    if (cacheResp) {
      return await cacheResp.arrayBuffer();
    } else {
      // Not in cache
      return await fetchUrl(plUrl);
    }
  } catch {
    // Cache API blocked / unsupported (e.g. firefox private)
    return fetchUrl(plUrl);
  }
}
