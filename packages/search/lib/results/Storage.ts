import Result from './Result';
import FieldInfo from './FieldInfo';

const storageMap: {
  [storage: string]: (
    results: Result[],
    baseUrl: string,
    storageParams: { baseName: string, [param: string]: any },
    fieldInfo: FieldInfo
  ) => Promise<void>
} = {};

storageMap.TextStorage = async (
  results: Result[],
  baseUrl: string,
  storageParams: { baseName: string, [param: string]: any },
): Promise<void> => {
  const { baseName, n: numDocsPerFile } = storageParams;
  const directoryUrl = `${baseUrl}/${baseName}`;

  const filePromises: { [fileName: number]: Promise<string> } = {};
  const lines: { [fileName: number]: string[] } = {};

  await Promise.all(results.map(async (result) => {
    const file = Math.floor((result.docId - 1) / numDocsPerFile) * numDocsPerFile + 1;
    filePromises[file] = filePromises[file] ?? fetch(`${directoryUrl}/${file}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'text/plain',
      },
    }).then((res) => res.text());

    lines[file] = lines[file] ?? (await filePromises[file]).split('\n');
    result.storages[baseName] = lines[file][(result.docId - 1) % numDocsPerFile];
  }));
};

storageMap.JsonStorage = async (
  results: Result[],
  baseUrl: string,
  storageParams: { baseName: string, [param: string]: any },
  fieldInfo: FieldInfo,
): Promise<void> => {
  const { baseName, n: numDocsPerFile } = storageParams;
  const directoryUrl = `${baseUrl}/${baseName}`;

  const filePromises: { [fileName: number]: Promise<(string | number)[][]> } = {};

  await Promise.all(results.map(async (result) => {
    const file = Math.floor((result.docId - 1) / numDocsPerFile) * numDocsPerFile;
    filePromises[file] = filePromises[file] ?? fetch(`${directoryUrl}/${file}.json`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    }).then((res) => res.json());

    const json = await filePromises[file];
    const texts = json[(result.docId - 1) % numDocsPerFile];
    const aggregatedTexts: { fieldName: string, text: string }[] = [];

    let currentField: number = texts[0] as number;
    for (let i = 0; i < texts.length; i += 1) {
      if (typeof texts[i] === 'number') {
        currentField = texts[i] as number;
      } else if (texts[i]) {
        aggregatedTexts.push({ fieldName: fieldInfo[currentField].name, text: texts[i] as string });
      }
    }

    result.storages[baseName] = aggregatedTexts;
  }));
};

class Storage {
  constructor(
    private storageType: string,
    private storageParams: { baseName: string, [param: string]: any },
    private baseUrl: string,
    private fieldInfo: FieldInfo,
  ) {}

  async populate(results: Result[]) {
    await storageMap[this.storageType](results, this.baseUrl, this.storageParams, this.fieldInfo);
  }
}

export default Storage;
