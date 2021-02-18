abstract class Storage {
  constructor(
    protected readonly outputFolderPath: string,
    public readonly baseName: string,
  ) {}

  abstract add(fieldName: string, docId: number, text: string): void;

  abstract dump(): void;
}

export default Storage;
