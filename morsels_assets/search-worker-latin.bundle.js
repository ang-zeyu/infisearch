(()=>{var n,t,e,r,i,o={899:(n,t,e)=>{e.p=__morsWrkrUrl.replace(/#.*$/,"").replace(/\?.*$/,"").replace(/\/[^\/]+$/,"/")},777:(n,t,e)=>{"use strict";e.a(n,(async n=>{e.d(t,{get_new_searcher:()=>r.qS,get_query:()=>r.R1});var r=e(686),i=n([r]);r=(i.then?await i:i)[0]}))},686:(n,t,e)=>{"use strict";e.a(n,(async r=>{e.d(t,{qS:()=>A,R1:()=>I,qt:()=>F,ug:()=>N,ff:()=>E,fW:()=>M,cI:()=>G,G6:()=>U,XP:()=>W,M1:()=>R,HT:()=>k,Wl:()=>C,$0:()=>D,pT:()=>Q,i9:()=>z,h4:()=>H,m_:()=>L,zr:()=>J,td:()=>K,Si:()=>V,lC:()=>X,vC:()=>Y,sv:()=>Z,j2:()=>nn,NA:()=>tn,Gn:()=>en,FP:()=>rn,Gt:()=>on,fs:()=>cn,rp:()=>_n,I7:()=>un,j1:()=>fn,o$:()=>sn,TL:()=>bn,Ge:()=>an,ID:()=>ln,Zn:()=>wn,fg:()=>dn,vz:()=>gn,Rb:()=>yn,_3:()=>hn,cN:()=>pn,OF:()=>mn,xB:()=>vn,KQ:()=>jn,vm:()=>xn,GW:()=>On,_8:()=>An,zL:()=>In,uv:()=>Sn,ry:()=>$n,Ns:()=>Bn,_9:()=>qn,oo:()=>Pn,d8:()=>Tn,My:()=>Fn,jV:()=>Nn,pi:()=>En,EF:()=>Mn,cb:()=>Gn,Ip:()=>Un,Oo:()=>Wn,Cp:()=>Rn,$q:()=>kn,fY:()=>Cn,Or:()=>Dn,oH:()=>Qn,E4:()=>zn});var i=e(945);n=e.hmd(n);var o=r([i]);i=(o.then?await o:o)[0];const c=new Array(32).fill(void 0);function _(n){return c[n]}c.push(void 0,null,!0,!1);let u=0,f=null;function s(){return null!==f&&f.buffer===i.memory.buffer||(f=new Uint8Array(i.memory.buffer)),f}let b=new("undefined"==typeof TextEncoder?(0,n.require)("util").TextEncoder:TextEncoder)("utf-8");const a="function"==typeof b.encodeInto?function(n,t){return b.encodeInto(n,t)}:function(n,t){const e=b.encode(n);return t.set(e),{read:n.length,written:e.length}};function l(n,t,e){if(void 0===e){const e=b.encode(n),r=t(e.length);return s().subarray(r,r+e.length).set(e),u=e.length,r}let r=n.length,i=t(r);const o=s();let c=0;for(;c<r;c++){const t=n.charCodeAt(c);if(t>127)break;o[i+c]=t}if(c!==r){0!==c&&(n=n.slice(c)),i=e(i,r,r=c+3*n.length);const t=s().subarray(i+c,i+r);c+=a(n,t).written}return u=c,i}function w(n){return null==n}let d=null;function g(){return null!==d&&d.buffer===i.memory.buffer||(d=new Int32Array(i.memory.buffer)),d}let y=c.length;function h(n){const t=_(n);return function(n){n<36||(c[n]=y,y=n)}(n),t}let p=null;function m(n){y===c.length&&c.push(c.length+1);const t=y;return y=c[t],c[t]=n,t}let v=new("undefined"==typeof TextDecoder?(0,n.require)("util").TextDecoder:TextDecoder)("utf-8",{ignoreBOM:!0,fatal:!0});function j(n,t){return v.decode(s().subarray(n,n+t))}function x(n){const t=typeof n;if("number"==t||"boolean"==t||null==n)return`${n}`;if("string"==t)return`"${n}"`;if("symbol"==t){const t=n.description;return null==t?"Symbol":`Symbol(${t})`}if("function"==t){const t=n.name;return"string"==typeof t&&t.length>0?`Function(${t})`:"Function"}if(Array.isArray(n)){const t=n.length;let e="[";t>0&&(e+=x(n[0]));for(let r=1;r<t;r++)e+=", "+x(n[r]);return e+="]",e}const e=/\[object ([^\]]+)\]/.exec(toString.call(n));let r;if(!(e.length>1))return toString.call(n);if(r=e[1],"Object"==r)try{return"Object("+JSON.stringify(n)+")"}catch(n){return"Object"}return n instanceof Error?`${n.name}: ${n.message}\n${n.stack}`:r}function O(n,t,e){i._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h8320177d520d82af(n,t,m(e))}function A(n,t,e){return h(i.get_new_searcher(m(n),m(t),m(e)))}function I(n,t){const e=l(t,i.__wbindgen_malloc,i.__wbindgen_realloc),r=u;return h(i.get_query(n,e,r))}function S(n,t){try{return n.apply(this,t)}catch(n){i.__wbindgen_exn_store(m(n))}}v.decode();const $=new Uint32Array(2),B=new BigUint64Array($.buffer),q=new BigInt64Array($.buffer);class P{static __wrap(n){const t=Object.create(P.prototype);return t.ptr=n,t}__destroy_into_raw(){const n=this.ptr;return this.ptr=0,n}free(){const n=this.__destroy_into_raw();i.__wbg_query_free(n)}get_next_n(n){return h(i.query_get_next_n(this.ptr,n))}get_query_parts(){return h(i.query_get_query_parts(this.ptr))}get_searched_terms(){return h(i.query_get_searched_terms(this.ptr))}}class T{static __wrap(n){const t=Object.create(T.prototype);return t.ptr=n,t}__destroy_into_raw(){const n=this.ptr;return this.ptr=0,n}free(){const n=this.__destroy_into_raw();i.__wbg_searcher_free(n)}get_ptr(){return i.searcher_get_ptr(this.ptr)}}function F(n,t){const e=_(t),r="string"==typeof e?e:void 0;var o=w(r)?0:l(r,i.__wbindgen_malloc,i.__wbindgen_realloc),c=u;g()[n/4+1]=c,g()[n/4+0]=o}function N(n){h(n)}function E(n){return m(P.__wrap(n))}function M(n){return"bigint"==typeof _(n)}function G(n){return m(T.__wrap(n))}function U(n){const t=h(n).original;if(1==t.cnt--)return t.a=0,!0;return!1}function W(n){return void 0===_(n)}function R(n,t){const e=_(t),r="number"==typeof e?e:void 0;(null!==p&&p.buffer===i.memory.buffer||(p=new Float64Array(i.memory.buffer)),p)[n/8+1]=w(r)?0:r,g()[n/4+0]=!w(r)}function k(n){const t=_(n);return"boolean"==typeof t?t?1:0:2}function C(n){const t=_(n);return"object"==typeof t&&null!==t}function D(n,t){$[0]=n,$[1]=t;const e=q[0];return m(BigInt(e))}function Q(n){return m(n)}function z(n,t){$[0]=n,$[1]=t;const e=B[0];return m(BigInt(e))}function H(n,t){return m(j(n,t))}function L(n){return m(_(n))}function J(n){return null===_(n)}function K(n,t){const e=BigInt(_(t));q[0]=e;const r=$[0],i=$[1];g()[n/4+1]=i,g()[n/4+0]=r}function V(n,t){const e=BigInt(_(t));B[0]=e;const r=$[0],i=$[1];g()[n/4+1]=i,g()[n/4+0]=r}function X(n,t){return m(_(n)[h(t)])}function Y(n,t,e){_(n)[h(t)]=h(e)}function Z(){return S((function(n){return m(_(n).caches)}),arguments)}function nn(n,t,e){return m(_(n).fetch(j(t,e)))}function tn(n,t,e){return m(_(n).open(j(t,e)))}function en(n){return _(n)instanceof Response}function rn(){return S((function(n){return m(_(n).arrayBuffer())}),arguments)}function on(n){return _(n)instanceof Cache}function cn(n,t,e){return m(_(n).match(j(t,e)))}function _n(n,t){return m(_(n)[t>>>0])}function un(n){return _(n).length}function fn(){return m(new Array)}function sn(n){return"function"==typeof _(n)}function bn(n,t){return m(new Function(j(n,t)))}function an(n){return m(_(n).next)}function ln(){return S((function(n){return m(_(n).next())}),arguments)}function wn(n){return _(n).done}function dn(n){return m(_(n).value)}function gn(){return m(Symbol.iterator)}function yn(){return S((function(n,t){return m(Reflect.get(_(n),_(t)))}),arguments)}function hn(){return S((function(n,t){return m(_(n).call(_(t)))}),arguments)}function pn(){return m(new Object)}function mn(){return S((function(){return m(self.self)}),arguments)}function vn(){return S((function(){return m(window.window)}),arguments)}function jn(){return S((function(){return m(globalThis.globalThis)}),arguments)}function xn(){return S((function(){return m(e.g.global)}),arguments)}function On(n,t,e){_(n)[t>>>0]=h(e)}function An(n){return Array.isArray(_(n))}function In(n){return _(n)instanceof ArrayBuffer}function Sn(n,t){return m(new Error(j(n,t)))}function $n(){return S((function(n,t,e){return m(_(n).call(_(t),_(e)))}),arguments)}function Bn(n){return Number.isSafeInteger(_(n))}function qn(n){return m(Object.entries(_(n)))}function Pn(n,t){return Object.is(_(n),_(t))}function Tn(n,t){try{var e={a:n,b:t};const r=new Promise(((n,t)=>{const r=e.a;e.a=0;try{return function(n,t,e,r){i.wasm_bindgen__convert__closures__invoke2_mut__h0c4af25e1d2ac3b9(n,t,m(e),m(r))}(r,e.b,n,t)}finally{e.a=r}}));return m(r)}finally{e.a=e.b=0}}function Fn(n){return m(Promise.resolve(_(n)))}function Nn(n,t){return m(_(n).then(_(t)))}function En(n,t,e){return m(_(n).then(_(t),_(e)))}function Mn(n){return m(_(n).buffer)}function Gn(n){return m(new Uint8Array(_(n)))}function Un(n,t,e){_(n).set(_(t),e>>>0)}function Wn(n){return _(n).length}function Rn(n){return _(n)instanceof Uint8Array}function kn(){return S((function(n,t){return Reflect.has(_(n),_(t))}),arguments)}function Cn(n,t){const e=l(x(_(t)),i.__wbindgen_malloc,i.__wbindgen_realloc),r=u;g()[n/4+1]=r,g()[n/4+0]=e}function Dn(n,t){throw new Error(j(n,t))}function Qn(){return m(i.memory)}function zn(n,t,e){const r=function(n,t,e,r){const o={a:n,b:t,cnt:1,dtor:e},c=(...n)=>{o.cnt++;const t=o.a;o.a=0;try{return r(t,o.b,...n)}finally{0==--o.cnt?i.__wbindgen_export_2.get(o.dtor)(t,o.b):o.a=t}};return c.original=o,c}(n,t,125,O);return m(r)}}))},945:(n,t,e)=>{"use strict";var r=([r])=>e.v(t,n.id,"ec0a0daa4371bff1778c",{"./index_bg.js":{__wbindgen_string_get:r.qt,__wbindgen_object_drop_ref:r.ug,__wbg_query_new:r.ff,__wbindgen_is_bigint:r.fW,__wbg_searcher_new:r.cI,__wbindgen_cb_drop:r.G6,__wbindgen_is_undefined:r.XP,__wbindgen_number_get:r.M1,__wbindgen_boolean_get:r.HT,__wbindgen_is_object:r.Wl,__wbg_BigInt_73b2c10d8e6eb5a5:r.$0,__wbindgen_number_new:r.pT,__wbg_BigInt_1a499fbb5f402f4c:r.i9,__wbindgen_string_new:r.h4,__wbindgen_object_clone_ref:r.m_,__wbindgen_is_null:r.zr,__wbg_BigInt_4365947136b5327c:r.td,__wbg_BigInt_6b6f34a01a71ad51:r.Si,__wbg_get_093fe3cdafaf8976:r.lC,__wbg_set_e93b31d47b90bff6:r.vC,__wbg_caches_3efbf43695d369e8:r.sv,__wbg_fetch_8df5fcf7dd9fd853:r.j2,__wbg_open_52c5eb54032958d7:r.NA,__wbg_instanceof_Response_ccfeb62399355bcd:r.Gn,__wbg_arrayBuffer_5a99283a3954c850:r.FP,__wbg_instanceof_Cache_47b072f80dd516f7:r.Gt,__wbg_match_739a4765298f04bf:r.fs,__wbg_get_590a2cd912f2ae46:r.rp,__wbg_length_2cd798326f2cc4c1:r.I7,__wbg_new_94fb1279cf6afea5:r.j1,__wbindgen_is_function:r.o$,__wbg_newnoargs_e23b458e372830de:r.TL,__wbg_next_cabb70b365520721:r.Ge,__wbg_next_bf3d83fc18df496e:r.ID,__wbg_done_040f966faa9a72b3:r.Zn,__wbg_value_419afbd9b9574c4c:r.fg,__wbg_iterator_4832ef1f15b0382b:r.vz,__wbg_get_a9cab131e3152c49:r.Rb,__wbg_call_ae78342adc33730a:r._3,__wbg_new_36359baae5a47e27:r.cN,__wbg_self_99737b4dcdf6f0d8:r.OF,__wbg_window_9b61fbbf3564c4fb:r.xB,__wbg_globalThis_8e275ef40caea3a3:r.KQ,__wbg_global_5de1e0f82bddcd27:r.vm,__wbg_set_561aac756158708c:r.GW,__wbg_isArray_6721f2e508996340:r._8,__wbg_instanceof_ArrayBuffer_b81b40c2ae0ab898:r.zL,__wbg_new_3047bf4b4f02b802:r.uv,__wbg_call_3ed288a247f13ea5:r.ry,__wbg_isSafeInteger_c87467ed96815119:r.Ns,__wbg_entries_aaf7a1fbe90f014a:r._9,__wbg_is_40969b082b54c84d:r.oo,__wbg_new_37705eed627d5ed9:r.d8,__wbg_resolve_a9a87bdd64e9e62c:r.My,__wbg_then_ce526c837d07b68f:r.jV,__wbg_then_842e65b843962f56:r.pi,__wbg_buffer_7af23f65f6c64548:r.EF,__wbg_new_cc9018bd6f283b6f:r.cb,__wbg_set_f25e869e4565d2a2:r.Ip,__wbg_length_0acb1cf9bbaf8519:r.Oo,__wbg_instanceof_Uint8Array_edb92795fc0c63b4:r.Cp,__wbg_has_ce995ec88636803d:r.$q,__wbindgen_debug_string:r.fY,__wbindgen_throw:r.Or,__wbindgen_memory:r.oH,__wbindgen_closure_wrapper598:r.E4}});e.a(n,(n=>{var t=n([e(686)]);return t.then?t.then(r):r(t)}),1)}},c={};function _(n){var t=c[n];if(void 0!==t)return t.exports;var e=c[n]={id:n,loaded:!1,exports:{}};return o[n](e,e.exports,_),e.loaded=!0,e.exports}n="function"==typeof Symbol?Symbol("webpack then"):"__webpack_then__",t="function"==typeof Symbol?Symbol("webpack exports"):"__webpack_exports__",e=n=>{n&&(n.forEach((n=>n.r--)),n.forEach((n=>n.r--?n.r++:n())))},r=n=>!--n.r&&n(),i=(n,t)=>n?n.push(t):r(t),_.a=(o,c,_)=>{var u,f,s,b=_&&[],a=o.exports,l=!0,w=!1,d=(t,e,r)=>{w||(w=!0,e.r+=t.length,t.map(((t,i)=>t[n](e,r))),w=!1)},g=new Promise(((n,t)=>{s=t,f=()=>(n(a),e(b),b=0)}));g[t]=a,g[n]=(n,t)=>{if(l)return r(n);u&&d(u,n,t),i(b,n),g.catch(t)},o.exports=g,c((o=>{if(!o)return f();var c,_;u=(o=>o.map((o=>{if(null!==o&&"object"==typeof o){if(o[n])return o;if(o.then){var c=[];o.then((n=>{_[t]=n,e(c),c=0}));var _={};return _[n]=(n,t)=>(i(c,n),o.catch(t)),_}}var u={};return u[n]=n=>r(n),u[t]=o,u})))(o);var s=new Promise(((n,e)=>{(c=()=>n(_=u.map((n=>n[t])))).r=0,d(u,c,e)}));return c.r?s:_})).then(f,s),l=!1},_.d=(n,t)=>{for(var e in t)_.o(t,e)&&!_.o(n,e)&&Object.defineProperty(n,e,{enumerable:!0,get:t[e]})},_.g=function(){if("object"==typeof globalThis)return globalThis;try{return this||new Function("return this")()}catch(n){if("object"==typeof window)return window}}(),_.hmd=n=>((n=Object.create(n)).children||(n.children=[]),Object.defineProperty(n,"exports",{enumerable:!0,set:()=>{throw new Error("ES Modules may not assign module.exports or exports.*, Use ESM export syntax, instead: "+n.id)}}),n),_.o=(n,t)=>Object.prototype.hasOwnProperty.call(n,t),_.v=(n,t,e,r)=>{var i=fetch(_.p+""+e+".module.wasm");return"function"==typeof WebAssembly.instantiateStreaming?WebAssembly.instantiateStreaming(i,r).then((t=>Object.assign(n,t.instance.exports))):i.then((n=>n.arrayBuffer())).then((n=>WebAssembly.instantiate(n,r))).then((t=>Object.assign(n,t.instance.exports)))},_.p="/",(()=>{"use strict";_(899);class n{constructor(n,t,e){this.t=n,this.i=t,this._=e}u(n){return this._.get_next_n(n)}l(){this._.free()}}var t=function(n,t,e,r){return new(e||(e=Promise))((function(i,o){function c(n){try{u(r.next(n))}catch(n){o(n)}}function _(n){try{u(r.throw(n))}catch(n){o(n)}}function u(n){var t;n.done?i(n.value):(t=n.value,t instanceof e?t:new e((function(n){n(t)}))).then(c,_)}u((r=r.apply(n,t||[])).next())}))};class e{constructor(n){this.h=n,this.m=Object.create(null)}j(e,r){return t(this,void 0,void 0,(function*(){const t=yield this.O.get_query(this.A.get_ptr(),e);return this.m[e]=this.m[e]||{},this.m[e][r]=new n(t.get_searched_terms(),t.get_query_parts(),t),this.m[e][r]}))}I(n,t,e){return this.m[n][t].u(e)}S(n,t){this.m[n][t]&&this.m[n][t].l(),delete this.m[n][t],0===Object.keys(this.m[n]).length&&delete this.m[n]}$(n,e){return t(this,void 0,void 0,(function*(){const[t,r]=n;this.O=yield e,this.A=yield this.O.get_new_searcher(this.h,t,r)}))}static B(n,r){return t(this,void 0,void 0,(function*(){const t=new e(n),i=n.searcherOptions.url,o=`${i}bitmap_docinfo_dicttable.json`,c=`${i}dictionary_string.json`;let _;try{_=yield caches.open(`morsels:${i}`)}catch(n){}const u=yield Promise.all([(_?_.match(o).then((n=>!n&&_.add(o))).then((()=>_.match(o))):fetch(o)).then((n=>n.arrayBuffer())),(_?_.match(c).then((n=>!n&&_.add(c))).then((()=>_.match(c))):fetch(c)).then((n=>n.arrayBuffer()))]);return yield t.$(u,r),t}))}}var r=function(n,t,e,r){return new(e||(e=Promise))((function(i,o){function c(n){try{u(r.next(n))}catch(n){o(n)}}function _(n){try{u(r.throw(n))}catch(n){o(n)}}function u(n){var t;n.done?i(n.value):(t=n.value,t instanceof e?t:new e((function(n){n(t)}))).then(c,_)}u((r=r.apply(n,t||[])).next())}))};!function(n){let t;onmessage=function(i){return r(this,void 0,void 0,(function*(){if(i.data.searcherOptions)t=yield e.B(i.data,n),postMessage({isSetupDone:!0});else if(i.data.query){const{query:n,queryId:e,n:r,isFree:o,isGetNextN:c}=i.data;if(o)t.S(n,e);else if(c){const i=t.I(n,e,r);postMessage({query:n,queryId:e,nextResults:i})}else{const r=yield t.j(n,e);postMessage({query:n,queryId:e,searchedTerms:r.t,queryParts:r.i})}}}))},postMessage("")}(Promise.resolve().then(_.bind(_,777)))})()})();