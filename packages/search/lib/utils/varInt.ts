/* eslint-disable no-bitwise */

const VALUE_MASK = 0x7f; // 0111 1111
const CONTINUATION_MASK = 0x80; // 1000 0000

function decodeVarInt(view: DataView, offset: number): { value: number, pos: number } {
  let currentValue = 0;
  let shiftAmount = 0;
  let pos = offset;

  for (; ; pos += 1) {
    const currentByte = view.getUint8(pos);
    const maskResult = VALUE_MASK & currentByte;
    currentValue += (maskResult << shiftAmount);

    if (CONTINUATION_MASK & currentByte) {
      break;
    } else {
      shiftAmount += 7;
    }
  }

  return { value: currentValue, pos: pos + 1 };
}

export default decodeVarInt;
