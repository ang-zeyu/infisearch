export default class JsonCache {
  linkToJsons: { [link: string]: Promise<any> } = Object.create(null);

  fetch(url: string): Promise<any> {
    if (!this.linkToJsons[url]) {
      this.linkToJsons[url] = fetch(url).then(res => res.json());
    }

    return this.linkToJsons[url];
  }
}
