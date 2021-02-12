/* eslint-disable no-bitwise */
const VALUE_MASK = 0x7f; // 0111 1111
const CONTINUATION_MASK = 0x80; // 1000 0000

class VarInt {
  value: Buffer;

  constructor(value: number) {
    const bytes: number[] = [];

    let newValue = value;

    while (newValue > 0) {
      const lastSevenBits = newValue & VALUE_MASK;
      newValue >>= 7;
      if (newValue > 0) {
        bytes.push(lastSevenBits);
      } else {
        bytes.push(lastSevenBits | CONTINUATION_MASK);
        break;
      }
    }

    this.value = Buffer.from(bytes);
  }
}

export default VarInt;
