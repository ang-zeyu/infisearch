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

export default class JsonCache {
  constructor(private cache: Cache) {}

  linkToJsons: { [link: string]: Promise<any> } = Object.create(null);

  async cacheJson(url: string) {
    let response: Response;
    if (this.cache) {
      response = await this.cache.match(url);
      if (!response) {
        throttle(async () => {
          await this.cache.add(url);
          response = await this.cache.match(url);
          this.linkToJsons[url] = response.json();
        });
      }
    } else {
      throttle(async () => {
        response = await fetch(url);
        this.linkToJsons[url] = response.json();
      });
    }
  }

  async cacheUrl(url: string) {
    if (this.cache) {
      const response = await this.cache.match(url);
      if (!response) {
        throttle(() => this.cache.add(url));
      }
    }
  }

  getJson(url: string): Promise<any> {
    if (!this.linkToJsons[url]) {
      this.linkToJsons[url] = fetch(url).then(res => res.json());
    }

    return this.linkToJsons[url];
  }
}
