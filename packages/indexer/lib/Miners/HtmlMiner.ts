import * as cheerio from 'cheerio';

import Miner from './Miner';
import Field from './Fields/Field';
import TextStorage from './Fields/TextStorage';
import JsonStorage from './Fields/JsonStorage';

const WHITESPACE = new RegExp('\\s+', 'g');

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
  'title',
];

const blockHtmlElementsSet = new Set(blockHtmlElements);

class HtmlMiner extends Miner {
  constructor(outputFolderPath) {
    const headingBodyStorage = new JsonStorage(outputFolderPath, { baseName: 'text', n: 1 });
    super(outputFolderPath, [
      new Field('title', 0.5, new TextStorage(outputFolderPath, { baseName: 'title', n: 100 })),
      new Field('heading', 0.3, headingBodyStorage),
      new Field('body', 0.2, headingBodyStorage),
      new Field('headingLink', 0, headingBodyStorage),
      new Field('link', 0, new TextStorage(outputFolderPath, { baseName: 'link', n: 100 })),
    ]);
  }

  private indexEl($: any, el: any, fields: { fieldName: string, text: string }[]): void {
    $(el).children().each((i, child) => {
      this.indexEl($, child, fields);
    });

    if (!blockHtmlElementsSet.has(el.name)) {
      return;
    }

    let fieldName;
    switch (el.name) {
      case 'title':
        fieldName = 'title';
        break;
      case 'h1':
      case 'h2':
      case 'h3':
      case 'h4':
      case 'h5':
      case 'h6':
      {
        const hrefEl = $(el).find('[href]')[0];
        const link = hrefEl && hrefEl.attribs.href;
        if (link) {
          fields.push({ fieldName: 'headingLink', text: link });
        }

        fieldName = 'heading';
        break;
      }
      default:
        fieldName = 'body';
    }

    const elTxt = $(el).text().replace(WHITESPACE, ' ');
    $(el).text('');

    fields.push({ fieldName, text: elTxt });
  }

  indexHtmlDoc(link: string, htmlSource: string) {
    const fields: { fieldName: string, text: string }[] = [];
    fields.push({ fieldName: 'link', text: link });

    const $ = cheerio.load(htmlSource);
    this.indexEl($, $('html')[0], fields);

    this.add(fields);
  }
}

export default HtmlMiner;
