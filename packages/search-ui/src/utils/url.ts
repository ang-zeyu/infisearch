export function parseURL(urlRaw: string): URL {
  if (urlRaw.startsWith('/')) {
    return new URL((new URL(window.location.href)).origin + urlRaw);
  }
  return new URL(urlRaw);
}
