import Storage from './Storage';

class Field {
  public id: number;

  constructor(
    public readonly name: string,
    public readonly weight: number,
    public readonly storage: Storage,
  ) {}

  add(docId: number, text: string): void {
    this.storage.add(this.id, docId, text);
  }

  dump(): void {
    this.storage.dump();
  }
}

export default Field;
