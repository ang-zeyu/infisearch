import * as wasm from '../../../librarian_search/pkg/index_bg.wasm';

export default async function fetchMultipleArrayBuffers(urlsRaw: string, ptr: number) {
  const urls = JSON.parse(urlsRaw);
  const wasmModule = await wasm;/*
  console.log(wasmModule);
  console.log(`${urlsRaw} ${ptr}`); */

  const ptrs: number[] = await Promise.all(urls.map(async (url) => {
    const arrayBuffer = await (await fetch(url)).arrayBuffer();
    const uInt8Array = new Uint8Array(arrayBuffer);
    // eslint-disable-next-line no-underscore-dangle
    const wasmBufferPtr = wasmModule.__wbindgen_malloc(uInt8Array.byteLength);
    (new Uint8Array(wasmModule.memory.buffer)).set(uInt8Array, wasmBufferPtr);

    return wasmBufferPtr;
  }));

  const arrayBuf = new ArrayBuffer(8);
  new Uint32Array(arrayBuf).set(ptrs);

  new Uint8Array(wasmModule.memory.buffer).set(new Uint8Array(arrayBuf), ptr);
}
