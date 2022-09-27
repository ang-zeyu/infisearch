import { MorselsConfig } from '../results/Config';

export function getFieldUrl(
  baseUrl: string,
  docId: number,
  config: MorselsConfig,
): string {
  const { numDocsPerStore, numStoresPerDir, indexingConfig } = config;
  const { numDocsPerBlock } = indexingConfig;
  const fileNumber = Math.floor(docId / numDocsPerStore);
  const blockNumber = Math.floor(docId / numDocsPerBlock);
  const dirNumber = Math.floor(fileNumber / numStoresPerDir);

  return `${baseUrl}field_store/${dirNumber}/${fileNumber}--${blockNumber}.json`;
}
