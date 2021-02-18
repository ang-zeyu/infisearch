import * as cheerio from 'cheerio';

import Miner from './Miner';
import Field from './Fields/Field';
import CombinedFileStorage from './Fields/CombinedFileStorage';
import SingleFileStorage from './Fields/SingleFileStorage';

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
    super(outputFolderPath, {
      title: new Field('title', 1.5, new CombinedFileStorage(outputFolderPath, 'title')),
      heading: new Field('heading', 1.2, new CombinedFileStorage(outputFolderPath, 'heading')),
      body: new Field('body', 1, new SingleFileStorage(outputFolderPath, 'body')),
      link: new Field('link', 0, new CombinedFileStorage(outputFolderPath, 'link')),
    });
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
        fieldName = 'heading';
        break;
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

    fields.push({ fieldName: 'title', text: '' });
    fields.push({ fieldName: 'heading', text: '' });

    const $ = cheerio.load(htmlSource);
    this.indexEl($, $('html')[0], fields);

    this.add(fields);
  }
}

export default HtmlMiner;
