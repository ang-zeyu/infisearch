export async function fetchPl(plName, numPlsPerDir, baseUrl, plLazyCacheThreshold) {
  const plUrl = `${baseUrl}pl_${Math.floor(plName / numPlsPerDir)}/pl_${plName}.mls`;
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
      const buf = await fetchUrl(plUrl);

      if (buf.byteLength >= plLazyCacheThreshold) {
        cache.add(plUrl);
      }

      return buf;
    }
  } catch {
    // Cache API blocked / unsupported (e.g. firefox private)
    return fetchUrl(plUrl);
  }
}
