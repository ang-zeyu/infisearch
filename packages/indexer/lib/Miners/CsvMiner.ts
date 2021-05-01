// eslint-disable-next-line import/no-extraneous-dependencies
import * as parse from 'csv-parse/lib/sync';

import Miner from './Miner';
import Field from './Fields/Field';
import TextStorage from './Fields/TextStorage';
import JsonStorage from './Fields/JsonStorage';

class CsvMiner extends Miner {
  constructor(outputFolderPath) {
    const headingBodyStorage = new JsonStorage(outputFolderPath, { baseName: 'text', n: 1 });
    super(outputFolderPath, [
      new Field('title', 0.2, new TextStorage(outputFolderPath, { baseName: 'title', n: 100 })),
      new Field('heading', 0.3, headingBodyStorage),
      new Field('body', 0.5, headingBodyStorage),
      new Field('headingLink', 0, headingBodyStorage),
      new Field('link', 0, new TextStorage(outputFolderPath, { baseName: 'link', n: 100 })),
    ]);
  }

  indexCsvDoc(link: string, csvRaw: string) {
    const records = parse(csvRaw, {
      columns: true,
    });

    records.forEach((record) => {
      const fields: { fieldName: string, text: string }[] = [];
      fields.push({ fieldName: 'link', text: 'dummylink' });
      fields.push({ fieldName: 'title', text: record.title });
      fields.push({ fieldName: 'body', text: record.content });

      this.add(fields);
    });
  }
}

export default CsvMiner;
