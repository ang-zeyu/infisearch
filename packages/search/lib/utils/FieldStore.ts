import { InfiConfig } from '../results/Config';

export function getFieldUrl(
  baseUrl: string,
  docId: number,
  config: InfiConfig,
): string {
  const { numDocsPerStore, numStoresPerDir, indexingConfig, indexVer } = config;
  const { numDocsPerBlock } = indexingConfig;
  const fileNumber = Math.floor(docId / numDocsPerStore);
  const blockNumber = Math.floor(docId / numDocsPerBlock);
  const dirNumber = Math.floor(fileNumber / numStoresPerDir);

  return `${baseUrl}${indexVer}/field_store/${dirNumber}/${fileNumber}--${blockNumber}.json`;
}
