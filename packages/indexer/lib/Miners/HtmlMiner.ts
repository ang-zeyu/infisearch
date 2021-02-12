import * as cheerio from 'cheerio';

import Miner from './Miner';

const blockHtmlElements : string[] = [
  'address',
  'article',
  'aside',
  'blockquote',
  'figcaption',
  'figure',
  'footer',
  'form',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'header',
  'li',
  'main',
  'nav',
  'p',
  'div',
  'section',
  'td',
];

const blockHtmlElementsSet = new Set(blockHtmlElements);

class HtmlMiner extends Miner {
  // eslint-disable-next-line @typescript-eslint/no-useless-constructor
  constructor(outputFolderPath) {
    super(outputFolderPath);
  }

  private indexEl($: any, el: any, fields: { [fieldName: string]: string[] }): void {
    $(el).children().each((i, child) => {
      this.indexEl($, child, fields);
    });

    if (!blockHtmlElementsSet.has(el.name)) {
      return;
    }

    fields[el.name] = fields[el.name] ?? [];

    const elTxt = $(el).text();
    $(el).text('');
    fields[el.name].push(elTxt);
  }

  indexHtmlDoc(link: string, htmlSource: string) {
    const $ = cheerio.load(htmlSource);
    const serp: string = $.root().text();

    const fields: { [fieldName: string]: string[] } = Object.create(null);
    this.indexEl($, $('html')[0], fields);

    this.add(link, serp, fields);
  }
}

export default HtmlMiner;
