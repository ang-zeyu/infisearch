class DictionaryEntry {
  constructor(
    public docFreq: number,
    public postingsFileName: number,
    public postingsFileOffset: number,
  ) {}
}

export default DictionaryEntry;
