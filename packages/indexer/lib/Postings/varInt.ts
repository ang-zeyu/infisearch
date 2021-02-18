/* eslint-disable no-bitwise */
const VALUE_MASK = 0x7f; // 0111 1111
const CONTINUATION_MASK = 0x80; // 1000 0000

function getVarInt(value: number): Buffer {
  const bytes: number[] = [];

  let newValue = value;

  do {
    const lastSevenBits = newValue & VALUE_MASK;
    newValue >>= 7;
    if (newValue > 0) {
      bytes.push(lastSevenBits);
    } else {
      bytes.push(lastSevenBits | CONTINUATION_MASK);
      break;
    }
  } while (newValue > 0);

  return Buffer.from(bytes);
}

export default getVarInt;
