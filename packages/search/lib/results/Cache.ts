let active = 0;
const networkQ: (() => Promise<any>)[] = [];

function pop() {
  if (networkQ.length) {
    networkQ.shift()().then(pop);
  }
}

async function throttle(startPromise: () => Promise<any>) {
  const wrapped = async () => {
    active += 1;
    await startPromise();
    active -= 1;
  };

  if (active >= 2) {
    networkQ.push(wrapped);
  } else {
    await wrapped();
    pop();
  }
}

export default class PersistentCache {
  constructor(private cache: Cache) {}

  private _mrlLinkToJsons: { [link: string]: Promise<any> } = Object.create(null);

  async _mrlCacheJson(url: string) {
    if (this.cache) {
      let cacheResp = await this.cache.match(url);
      if (cacheResp) {
        this._mrlLinkToJsons[url] = cacheResp.json();
      } else {
        throttle(async () => {
          await this.cache.add(url);
          cacheResp = await this.cache.match(url);
          this._mrlLinkToJsons[url] = cacheResp.json();
        });
      }
    } else {
      throttle(async () => {
        const response = await fetch(url);
        this._mrlLinkToJsons[url] = response.json();
      });
    }
  }

  async _mrlCacheUrl(url: string) {
    if (this.cache) {
      const response = await this.cache.match(url);
      if (!response) {
        throttle(() => this.cache.add(url));
      }
    }
  }

  getJson(url: string): Promise<any> {
    if (!this._mrlLinkToJsons[url]) {
      this._mrlLinkToJsons[url] = fetch(url).then(res => res.json());
    }

    return this._mrlLinkToJsons[url];
  }
}
