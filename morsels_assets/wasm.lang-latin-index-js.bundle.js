"use strict";(self.webpackChunkmorsels=self.webpackChunkmorsels||[]).push([[949],{777:(e,n,_)=>{_.a(e,(async e=>{_.r(n),_.d(n,{Query:()=>r.AE,Searcher:()=>r.sz,__wbg_arrayBuffer_b8937ed04beb0d36:()=>r.gV,__wbg_buffer_397eaa4d72ee94dd:()=>r.jp,__wbg_call_346669c262382ad7:()=>r.Ms,__wbg_call_888d259a5fefc347:()=>r.BT,__wbg_fetch_3a636c71a7d400b0:()=>r.ih,__wbg_globalThis_3f735a5746d41fbd:()=>r.ud,__wbg_global_1bc0b39582740e95:()=>r.PT,__wbg_instanceof_Response_e1b11afbefa5b563:()=>r.Yb,__wbg_length_1eb8fc608a0d4cdb:()=>r.A7,__wbg_new_a7ce447f15ff496f:()=>r.y4,__wbg_new_b1d61b5687f5e73a:()=>r.hq,__wbg_newnoargs_be86524d73f67598:()=>r.wg,__wbg_query_new:()=>r.ff,__wbg_resolve_d23068002f584f22:()=>r.zb,__wbg_searcher_new:()=>r.cI,__wbg_self_c6fbdfc2918d5e58:()=>r.JX,__wbg_set_969ad0a60e51d320:()=>r.YQ,__wbg_then_2fcac196782070cc:()=>r.Zp,__wbg_then_8c2d62e8ae5978f7:()=>r.v_,__wbg_window_baec038b5ab35c54:()=>r.xd,__wbindgen_cb_drop:()=>r.G6,__wbindgen_closure_wrapper1169:()=>r.AZ,__wbindgen_debug_string:()=>r.fY,__wbindgen_is_undefined:()=>r.XP,__wbindgen_json_parse:()=>r.t$,__wbindgen_json_serialize:()=>r.r1,__wbindgen_memory:()=>r.oH,__wbindgen_object_clone_ref:()=>r.m_,__wbindgen_object_drop_ref:()=>r.ug,__wbindgen_throw:()=>r.Or,get_new_searcher:()=>r.qS,get_query:()=>r.R1});var r=_(686),t=e([r]);r=(t.then?await t:t)[0]}))},686:(e,n,_)=>{_.a(e,(async r=>{_.d(n,{qS:()=>q,R1:()=>A,AE:()=>x,sz:()=>O,ug:()=>S,cI:()=>$,ff:()=>Y,G6:()=>k,t$:()=>P,r1:()=>z,ih:()=>B,Yb:()=>E,gV:()=>I,wg:()=>J,BT:()=>R,m_:()=>X,Ms:()=>Z,hq:()=>M,zb:()=>C,Zp:()=>F,v_:()=>Q,JX:()=>D,xd:()=>G,ud:()=>H,PT:()=>N,XP:()=>V,jp:()=>U,y4:()=>W,YQ:()=>K,A7:()=>L,fY:()=>ee,Or:()=>ne,oH:()=>_e,AZ:()=>re});var t=_(945);e=_.hmd(e);var c=r([t]);t=(c.then?await c:c)[0];const o=new Array(32).fill(void 0);function u(e){return o[e]}o.push(void 0,null,!0,!1);let i=o.length;function f(e){const n=u(e);return function(e){e<36||(o[e]=i,i=e)}(e),n}let a=new("undefined"==typeof TextDecoder?(0,e.require)("util").TextDecoder:TextDecoder)("utf-8",{ignoreBOM:!0,fatal:!0});a.decode();let b=null;function s(){return null!==b&&b.buffer===t.memory.buffer||(b=new Uint8Array(t.memory.buffer)),b}function g(e,n){return a.decode(s().subarray(e,e+n))}function d(e){i===o.length&&o.push(o.length+1);const n=i;return i=o[n],o[n]=e,n}let l=0;let w=new("undefined"==typeof TextEncoder?(0,e.require)("util").TextEncoder:TextEncoder)("utf-8");const h="function"==typeof w.encodeInto?function(e,n){return w.encodeInto(e,n)}:function(e,n){const _=w.encode(e);return n.set(_),{read:e.length,written:_.length}};function y(e,n,_){if(void 0===_){const _=w.encode(e),r=n(_.length);return s().subarray(r,r+_.length).set(_),l=_.length,r}let r=e.length,t=n(r);const c=s();let o=0;for(;o<r;o++){const n=e.charCodeAt(o);if(n>127)break;c[t+o]=n}if(o!==r){0!==o&&(e=e.slice(o)),t=_(t,r,r=o+3*e.length);const n=s().subarray(t+o,t+r);o+=h(e,n).written}return l=o,t}let p=null;function m(){return null!==p&&p.buffer===t.memory.buffer||(p=new Int32Array(t.memory.buffer)),p}function v(e){const n=typeof e;if("number"==n||"boolean"==n||null==e)return`${e}`;if("string"==n)return`"${e}"`;if("symbol"==n){const n=e.description;return null==n?"Symbol":`Symbol(${n})`}if("function"==n){const n=e.name;return"string"==typeof n&&n.length>0?`Function(${n})`:"Function"}if(Array.isArray(e)){const n=e.length;let _="[";n>0&&(_+=v(e[0]));for(let r=1;r<n;r++)_+=", "+v(e[r]);return _+="]",_}const _=/\[object ([^\]]+)\]/.exec(toString.call(e));let r;if(!(_.length>1))return toString.call(e);if(r=_[1],"Object"==r)try{return"Object("+JSON.stringify(e)+")"}catch(e){return"Object"}return e instanceof Error?`${e.name}: ${e.message}\n${e.stack}`:r}function j(e,n,_){t._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h06a02c5ac5937290(e,n,d(_))}function q(e){return f(t.get_new_searcher(d(e)))}function A(e,n){var _=y(n,t.__wbindgen_malloc,t.__wbindgen_realloc),r=l;return f(t.get_query(e,_,r))}function T(e,n){try{return e.apply(this,n)}catch(e){t.__wbindgen_exn_store(d(e))}}class x{static __wrap(e){const n=Object.create(x.prototype);return n.ptr=e,n}__destroy_into_raw(){const e=this.ptr;return this.ptr=0,e}free(){const e=this.__destroy_into_raw();t.__wbg_query_free(e)}get_next_n(e){return f(t.query_get_next_n(this.ptr,e))}get_query_parts(){return f(t.query_get_query_parts(this.ptr))}get_searched_terms(){return f(t.query_get_searched_terms(this.ptr))}}class O{static __wrap(e){const n=Object.create(O.prototype);return n.ptr=e,n}__destroy_into_raw(){const e=this.ptr;return this.ptr=0,e}free(){const e=this.__destroy_into_raw();t.__wbg_searcher_free(e)}get_ptr(){return t.searcher_get_ptr(this.ptr)}}function S(e){f(e)}function $(e){return d(O.__wrap(e))}function Y(e){return d(x.__wrap(e))}function k(e){const n=f(e).original;if(1==n.cnt--)return n.a=0,!0;return!1}function P(e,n){return d(JSON.parse(g(e,n)))}function z(e,n){const _=u(n);var r=y(JSON.stringify(void 0===_?null:_),t.__wbindgen_malloc,t.__wbindgen_realloc),c=l;m()[e/4+1]=c,m()[e/4+0]=r}function B(e,n,_){return d(u(e).fetch(g(n,_)))}function E(e){return u(e)instanceof Response}function I(){return T((function(e){return d(u(e).arrayBuffer())}),arguments)}function J(e,n){return d(new Function(g(e,n)))}function R(){return T((function(e,n){return d(u(e).call(u(n)))}),arguments)}function X(e){return d(u(e))}function Z(){return T((function(e,n,_){return d(u(e).call(u(n),u(_)))}),arguments)}function M(e,n){try{var _={a:e,b:n},r=new Promise(((e,n)=>{const r=_.a;_.a=0;try{return function(e,n,_,r){t.wasm_bindgen__convert__closures__invoke2_mut__h379562589f4b46dc(e,n,d(_),d(r))}(r,_.b,e,n)}finally{_.a=r}}));return d(r)}finally{_.a=_.b=0}}function C(e){return d(Promise.resolve(u(e)))}function F(e,n){return d(u(e).then(u(n)))}function Q(e,n,_){return d(u(e).then(u(n),u(_)))}function D(){return T((function(){return d(self.self)}),arguments)}function G(){return T((function(){return d(window.window)}),arguments)}function H(){return T((function(){return d(globalThis.globalThis)}),arguments)}function N(){return T((function(){return d(_.g.global)}),arguments)}function V(e){return void 0===u(e)}function U(e){return d(u(e).buffer)}function W(e){return d(new Uint8Array(u(e)))}function K(e,n,_){u(e).set(u(n),_>>>0)}function L(e){return u(e).length}function ee(e,n){var _=y(v(u(n)),t.__wbindgen_malloc,t.__wbindgen_realloc),r=l;m()[e/4+1]=r,m()[e/4+0]=_}function ne(e,n){throw new Error(g(e,n))}function _e(){return d(t.memory)}function re(e,n,_){var r=function(e,n,_,r){const c={a:e,b:n,cnt:1,dtor:_},o=(...e)=>{c.cnt++;const n=c.a;c.a=0;try{return r(n,c.b,...e)}finally{0==--c.cnt?t.__wbindgen_export_2.get(c.dtor)(n,c.b):c.a=n}};return o.original=c,o}(e,n,306,j);return d(r)}}))},945:(e,n,_)=>{var r=([r])=>_.v(n,e.id,"b07b81fe13c0961fecfe",{"./index_bg.js":{__wbindgen_object_drop_ref:r.ug,__wbg_searcher_new:r.cI,__wbg_query_new:r.ff,__wbindgen_cb_drop:r.G6,__wbindgen_json_parse:r.t$,__wbindgen_json_serialize:r.r1,__wbg_fetch_3a636c71a7d400b0:r.ih,__wbg_instanceof_Response_e1b11afbefa5b563:r.Yb,__wbg_arrayBuffer_b8937ed04beb0d36:r.gV,__wbg_newnoargs_be86524d73f67598:r.wg,__wbg_call_888d259a5fefc347:r.BT,__wbindgen_object_clone_ref:r.m_,__wbg_call_346669c262382ad7:r.Ms,__wbg_new_b1d61b5687f5e73a:r.hq,__wbg_resolve_d23068002f584f22:r.zb,__wbg_then_2fcac196782070cc:r.Zp,__wbg_then_8c2d62e8ae5978f7:r.v_,__wbg_self_c6fbdfc2918d5e58:r.JX,__wbg_window_baec038b5ab35c54:r.xd,__wbg_globalThis_3f735a5746d41fbd:r.ud,__wbg_global_1bc0b39582740e95:r.PT,__wbindgen_is_undefined:r.XP,__wbg_buffer_397eaa4d72ee94dd:r.jp,__wbg_new_a7ce447f15ff496f:r.y4,__wbg_set_969ad0a60e51d320:r.YQ,__wbg_length_1eb8fc608a0d4cdb:r.A7,__wbindgen_debug_string:r.fY,__wbindgen_throw:r.Or,__wbindgen_memory:r.oH,__wbindgen_closure_wrapper1169:r.AZ}});_.a(e,(e=>{var n=e([_(686)]);return n.then?n.then(r):r(n)}),1)}}]);