(()=>{var e,t,r,n,s,i={187:(e,t,r)=>{var n={"./lang-ascii/index.js":[519,964],"./lang-chinese/index.js":[312,635],"./lang-latin/index.js":[777,949]};function s(e){if(!r.o(n,e))return Promise.resolve().then((()=>{var t=new Error("Cannot find module '"+e+"'");throw t.code="MODULE_NOT_FOUND",t}));var t=n[e],s=t[0];return r.e(t[1]).then((()=>r(s)))}s.keys=()=>Object.keys(n),s.id=187,e.exports=s}},o={};function a(e){var t=o[e];if(void 0!==t)return t.exports;var r=o[e]={id:e,loaded:!1,exports:{}};return i[e](r,r.exports,a),r.loaded=!0,r.exports}a.m=i,e="function"==typeof Symbol?Symbol("webpack then"):"__webpack_then__",t="function"==typeof Symbol?Symbol("webpack exports"):"__webpack_exports__",r=e=>{e&&(e.forEach((e=>e.r--)),e.forEach((e=>e.r--?e.r++:e())))},n=e=>!--e.r&&e(),s=(e,t)=>e?e.push(t):n(t),a.a=(i,o,a)=>{var u,c,l,h=a&&[],f=i.exports,p=!0,d=!1,y=(t,r,n)=>{d||(d=!0,r.r+=t.length,t.map(((t,s)=>t[e](r,n))),d=!1)},m=new Promise(((e,t)=>{l=t,c=()=>(e(f),r(h),h=0)}));m[t]=f,m[e]=(e,t)=>{if(p)return n(e);u&&y(u,e,t),s(h,e),m.catch(t)},i.exports=m,o((i=>{if(!i)return c();var o,a;u=(i=>i.map((i=>{if(null!==i&&"object"==typeof i){if(i[e])return i;if(i.then){var o=[];i.then((e=>{a[t]=e,r(o),o=0}));var a={};return a[e]=(e,t)=>(s(o,e),i.catch(t)),a}}var u={};return u[e]=e=>n(e),u[t]=i,u})))(i);var l=new Promise(((e,r)=>{(o=()=>e(a=u.map((e=>e[t])))).r=0,y(u,o,r)}));return o.r?l:a})).then(c,l),p=!1},a.d=(e,t)=>{for(var r in t)a.o(t,r)&&!a.o(e,r)&&Object.defineProperty(e,r,{enumerable:!0,get:t[r]})},a.f={},a.e=e=>Promise.all(Object.keys(a.f).reduce(((t,r)=>(a.f[r](e,t),t)),[])),a.u=e=>({635:"wasm.lang-chinese-index-js",949:"wasm.lang-latin-index-js",964:"wasm.lang-ascii-index-js"}[e]+".bundle.js"),a.miniCssF=e=>{},a.g=function(){if("object"==typeof globalThis)return globalThis;try{return this||new Function("return this")()}catch(e){if("object"==typeof window)return window}}(),a.hmd=e=>((e=Object.create(e)).children||(e.children=[]),Object.defineProperty(e,"exports",{enumerable:!0,set:()=>{throw new Error("ES Modules may not assign module.exports or exports.*, Use ESM export syntax, instead: "+e.id)}}),e),a.o=(e,t)=>Object.prototype.hasOwnProperty.call(e,t),a.r=e=>{"undefined"!=typeof Symbol&&Symbol.toStringTag&&Object.defineProperty(e,Symbol.toStringTag,{value:"Module"}),Object.defineProperty(e,"__esModule",{value:!0})},a.v=(e,t,r,n)=>{var s=fetch(a.p+""+r+".module.wasm");return"function"==typeof WebAssembly.instantiateStreaming?WebAssembly.instantiateStreaming(s,n).then((t=>Object.assign(e,t.instance.exports))):s.then((e=>e.arrayBuffer())).then((e=>WebAssembly.instantiate(e,n))).then((t=>Object.assign(e,t.instance.exports)))},(()=>{var e;a.g.importScripts&&(e=a.g.location+"");var t=a.g.document;if(!e&&t&&(t.currentScript&&(e=t.currentScript.src),!e)){var r=t.getElementsByTagName("script");r.length&&(e=r[r.length-1].src)}if(!e)throw new Error("Automatic publicPath is not supported in this browser");e=e.replace(/#.*$/,"").replace(/\?.*$/,"").replace(/\/[^\/]+$/,"/"),a.p=e})(),(()=>{var e={658:1};a.f.i=(t,r)=>{e[t]||importScripts(a.p+a.u(t))};var t=self.webpackChunkmorsels=self.webpackChunkmorsels||[],r=t.push.bind(t);t.push=t=>{var[n,s,i]=t;for(var o in s)a.o(s,o)&&(a.m[o]=s[o]);for(i&&i(a);n.length;)e[n.pop()]=1;r(t)}})(),(()=>{"use strict";class e{constructor(e,t,r){this.searchedTerms=e,this.queryParts=t,this.query=r}getNextN(e){return this.query.get_next_n(e)}free(){this.query.free()}}var t=function(e,t,r,n){return new(r||(r=Promise))((function(s,i){function o(e){try{u(n.next(e))}catch(e){i(e)}}function a(e){try{u(n.throw(e))}catch(e){i(e)}}function u(e){var t;e.done?s(e.value):(t=e.value,t instanceof r?t:new r((function(e){e(t)}))).then(o,a)}u((n=n.apply(e,t||[])).next())}))};class r{constructor(e){this.config=e,this.workerQueries=Object.create(null)}processQuery(r,n){return t(this,void 0,void 0,(function*(){const t=yield this.wasmModule.get_query(this.wasmSearcher.get_ptr(),r);return this.workerQueries[r]=this.workerQueries[r]||{},this.workerQueries[r][n]=new e(t.get_searched_terms(),t.get_query_parts(),t),this.workerQueries[r][n]}))}getQueryNextN(e,t,r){return this.workerQueries[e][t].getNextN(r)}freeQuery(e,t){this.workerQueries[e][t]&&this.workerQueries[e][t].free(),delete this.workerQueries[e][t],0===Object.keys(this.workerQueries[e]).length&&delete this.workerQueries[e]}setupWasm(){return t(this,void 0,void 0,(function*(){const e=this.config.langConfig.lang;this.wasmModule=yield a(187)(`./lang-${e}/index.js`),this.wasmSearcher=yield this.wasmModule.get_new_searcher(this.config)}))}static setup(e){return t(this,void 0,void 0,(function*(){const t=new r(e);return yield t.setupWasm(),t}))}}var n=function(e,t,r,n){return new(r||(r=Promise))((function(s,i){function o(e){try{u(n.next(e))}catch(e){i(e)}}function a(e){try{u(n.throw(e))}catch(e){i(e)}}function u(e){var t;e.done?s(e.value):(t=e.value,t instanceof r?t:new r((function(e){e(t)}))).then(o,a)}u((n=n.apply(e,t||[])).next())}))};let s;onmessage=function(e){return n(this,void 0,void 0,(function*(){if(e.data.searcherOptions)s=yield r.setup(e.data),postMessage({isSetupDone:!0});else if(e.data.query){const{query:t,queryId:r,n,isFree:i,isGetNextN:o}=e.data;if(i)s.freeQuery(t,r);else if(o){const e=s.getQueryNextN(t,r,n);postMessage({query:t,queryId:r,nextResults:e})}else{const e=yield s.processQuery(t,r);postMessage({query:t,queryId:r,searchedTerms:e.searchedTerms,queryParts:e.queryParts})}}}))}})()})();