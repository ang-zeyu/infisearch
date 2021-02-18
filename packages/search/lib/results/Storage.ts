import Result from './Result';

const storageMap: {
  [storage: string]: (results: Result[], baseUrl: string, fieldName: string, baseFileName: string) => Promise<void>
} = {};

storageMap.SingleFileStorage = async (
  results: Result[],
  baseUrl: string,
  fieldName: string,
  baseFileName: string,
): Promise<void> => {
  const directoryUrl = `${baseUrl}/${baseFileName}`;
  await Promise.all(results.map(async (result) => {
    result.fields[fieldName] = await (await fetch(`${directoryUrl}/${result.docId}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'text/plain',
      },
    })).text();
  }));
};

storageMap.CombinedFileStorage = async (
  results: Result[],
  baseUrl: string,
  fieldName: string,
  baseFileName: string,
): Promise<void> => {
  const fileUrl = `${baseUrl}/${baseFileName}`;
  const lines = (await (await fetch(fileUrl, {
    method: 'GET',
    headers: {
      'Content-Type': 'text/plain',
    },
  })).text()).split('\n');

  results.forEach((result) => {
    result.fields[fieldName] = lines[result.docId - 1];
  });
};

export default storageMap;
