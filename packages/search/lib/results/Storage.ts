import Result from './Result';

const storageMap: {
  [storage: string]: (
    results: Result[],
    baseUrl: string,
    fieldName: string,
    storageParams: { [param: string]: any },
  ) => Promise<void>
} = {};

storageMap.TextStorage = async (
  results: Result[],
  baseUrl: string,
  fieldName: string,
  storageParams: { [param: string]: any },
): Promise<void> => {
  const directoryUrl = `${baseUrl}/${storageParams.baseName}`;
  const numDocsPerFile = storageParams.n;

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
    result.fields[fieldName] = lines[file][(result.docId - 1) % numDocsPerFile];
  }));
};

export default storageMap;
