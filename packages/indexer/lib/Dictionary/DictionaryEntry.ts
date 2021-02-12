class DictionaryEntry {
  constructor(
    public term: string,
    public docFreq: number,
    public postingsFileName: number,
    public postingsFileOffset: number,
    public postingsFileLength: number,
  ) {}
}

export default DictionaryEntry;
