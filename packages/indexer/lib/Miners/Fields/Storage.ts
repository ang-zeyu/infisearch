abstract class Storage {
  constructor(
    protected readonly outputFolderPath: string,
    public readonly params: { baseName: string },
  ) {}

  abstract add(fieldId: number, docId: number, text: string): void;

  abstract dump(): void;
}

export default Storage;
