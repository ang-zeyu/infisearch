# Content Security Policy

## WebAssembly CSP

InfiSearch runs using WebAssembly. If you are using a restrictive content security policy,  WebAssembly as a whole unfortunately currently requires adding the `script-src: 'unsafe-eval';` directive.

This error will show up in chrome for example as the following extremely detailed error message:


> Uncaught (in promise) CompileError: WebAssembly.instantiateStreaming():
> Refused to compile or instantiate WebAssembly module because 'unsafe-eval'
> is not an allowed source of script in the following Content Security Policy directive: '...'

Support for a more specific `script-src: 'wasm-unsafe-eval';` directive has landed in Chrome, Edge and Firefox, but is still pending in Safari.

## WebWorker CSP

InfiSearch also utilises a [blob URL](https://stackoverflow.com/questions/30864573/what-is-a-blob-url-and-why-it-is-used) to load its WebWorker. This shouldn't pose as much of a security concern since blob URLs can only be created by scripts already executing within the browser.

To whitelist this, add the `script-src: blob:;` directive.

## CDN CSP

Naturally, if you load InfiSearch assets from the CDN, you will also need to whitelist this in the `script-src: cdn.jsdelivr.net;` and `style-src: cdn.jsdelivr.net;` directives.
