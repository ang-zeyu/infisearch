"use strict";(self.webpackChunkmorsels=self.webpackChunkmorsels||[]).push([[949],{777:(e,n,_)=>{_.a(e,(async e=>{_.r(n),_.d(n,{Query:()=>t.AE,Searcher:()=>t.sz,__wbg_arrayBuffer_b7c95af83e1e2705:()=>t.s,__wbg_buffer_9e184d6f785de5ed:()=>t.zP,__wbg_call_3fc07b7d5fc9022d:()=>t.tw,__wbg_call_ba36642bd901572b:()=>t.qw,__wbg_fetch_b90bd6bfc2ff5f95:()=>t.I8,__wbg_fetch_eb9fd115eef29d0c:()=>t.dZ,__wbg_globalThis_e0d21cabc6630763:()=>t.md,__wbg_global_8463719227271676:()=>t.IF,__wbg_instanceof_Response_d61ff4c524b8dbc4:()=>t.mC,__wbg_length_2d56cb37075fcfb1:()=>t.uQ,__wbg_new_c143a4f563f78c4e:()=>t.Y4,__wbg_new_e8101319e4cf95fc:()=>t.td,__wbg_newnoargs_9fdd8f3961dd1bee:()=>t.UL,__wbg_newwithstr_07dc8adf8bcc4e86:()=>t.Ey,__wbg_query_new:()=>t.ff,__wbg_resolve_cae3d8f752f5db88:()=>t.Xb,__wbg_searcher_new:()=>t.cI,__wbg_self_bb69a836a72ec6e9:()=>t.tS,__wbg_set_e8ae7b27314e8b98:()=>t.Ct,__wbg_then_6c9a4bf55755f9b8:()=>t.KA,__wbg_then_c2361a9d5c9a4fcb:()=>t.g1,__wbg_window_3304fc4b414c9693:()=>t.R$,__wbindgen_cb_drop:()=>t.G6,__wbindgen_closure_wrapper1147:()=>t.DP,__wbindgen_debug_string:()=>t.fY,__wbindgen_is_undefined:()=>t.XP,__wbindgen_json_parse:()=>t.t$,__wbindgen_json_serialize:()=>t.r1,__wbindgen_memory:()=>t.oH,__wbindgen_object_clone_ref:()=>t.m_,__wbindgen_object_drop_ref:()=>t.ug,__wbindgen_throw:()=>t.Or,get_new_searcher:()=>t.qS,get_query:()=>t.R1});var t=_(686),r=e([t]);t=(r.then?await r:r)[0]}))},686:(e,n,_)=>{_.a(e,(async t=>{_.d(n,{qS:()=>x,R1:()=>j,AE:()=>O,sz:()=>S,ug:()=>$,cI:()=>I,ff:()=>P,t$:()=>C,r1:()=>E,G6:()=>R,dZ:()=>T,I8:()=>k,mC:()=>z,s:()=>F,Ey:()=>D,qw:()=>X,m_:()=>Y,UL:()=>U,tw:()=>B,Y4:()=>Q,Xb:()=>G,g1:()=>H,KA:()=>J,tS:()=>K,R$:()=>L,md:()=>N,IF:()=>Z,XP:()=>M,zP:()=>W,uQ:()=>V,td:()=>ee,Ct:()=>ne,fY:()=>_e,Or:()=>te,oH:()=>re,DP:()=>ce});var r=_(945);e=_.hmd(e);var c=t([r]);r=(c.then?await c:c)[0];const o=new Array(32).fill(void 0);function f(e){return o[e]}o.push(void 0,null,!0,!1);let u=o.length;function i(e){const n=f(e);return function(e){e<36||(o[e]=u,u=e)}(e),n}let b=new("undefined"==typeof TextDecoder?(0,e.require)("util").TextDecoder:TextDecoder)("utf-8",{ignoreBOM:!0,fatal:!0});b.decode();let s=null;function a(){return null!==s&&s.buffer===r.memory.buffer||(s=new Uint8Array(r.memory.buffer)),s}function d(e,n){return b.decode(a().subarray(e,e+n))}function g(e){u===o.length&&o.push(o.length+1);const n=u;return u=o[n],o[n]=e,n}let w=0;let l=new("undefined"==typeof TextEncoder?(0,e.require)("util").TextEncoder:TextEncoder)("utf-8");const h="function"==typeof l.encodeInto?function(e,n){return l.encodeInto(e,n)}:function(e,n){const _=l.encode(e);return n.set(_),{read:e.length,written:_.length}};function y(e,n,_){if(void 0===_){const _=l.encode(e),t=n(_.length);return a().subarray(t,t+_.length).set(_),w=_.length,t}let t=e.length,r=n(t);const c=a();let o=0;for(;o<t;o++){const n=e.charCodeAt(o);if(n>127)break;c[r+o]=n}if(o!==t){0!==o&&(e=e.slice(o)),r=_(r,t,t=o+3*e.length);const n=a().subarray(r+o,r+t);o+=h(e,n).written}return w=o,r}let p=null;function m(){return null!==p&&p.buffer===r.memory.buffer||(p=new Int32Array(r.memory.buffer)),p}function q(e){const n=typeof e;if("number"==n||"boolean"==n||null==e)return`${e}`;if("string"==n)return`"${e}"`;if("symbol"==n){const n=e.description;return null==n?"Symbol":`Symbol(${n})`}if("function"==n){const n=e.name;return"string"==typeof n&&n.length>0?`Function(${n})`:"Function"}if(Array.isArray(e)){const n=e.length;let _="[";n>0&&(_+=q(e[0]));for(let t=1;t<n;t++)_+=", "+q(e[t]);return _+="]",_}const _=/\[object ([^\]]+)\]/.exec(toString.call(e));let t;if(!(_.length>1))return toString.call(e);if(t=_[1],"Object"==t)try{return"Object("+JSON.stringify(e)+")"}catch(e){return"Object"}return e instanceof Error?`${e.name}: ${e.message}\n${e.stack}`:t}function v(e,n,_){r._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h1a7e736187f0625f(e,n,g(_))}function x(e){return i(r.get_new_searcher(g(e)))}function j(e,n){var _=y(n,r.__wbindgen_malloc,r.__wbindgen_realloc),t=w;return i(r.get_query(e,_,t))}function A(e,n){try{return e.apply(this,n)}catch(e){r.__wbindgen_exn_store(g(e))}}class O{static __wrap(e){const n=Object.create(O.prototype);return n.ptr=e,n}__destroy_into_raw(){const e=this.ptr;return this.ptr=0,e}free(){const e=this.__destroy_into_raw();r.__wbg_query_free(e)}get is_free_text_query(){return 0!==r.__wbg_get_query_is_free_text_query(this.ptr)}set is_free_text_query(e){r.__wbg_set_query_is_free_text_query(this.ptr,e)}get_next_n(e){return i(r.query_get_next_n(this.ptr,e))}get_query_parts(){return i(r.query_get_query_parts(this.ptr))}get_searched_terms(){return i(r.query_get_searched_terms(this.ptr))}}class S{static __wrap(e){const n=Object.create(S.prototype);return n.ptr=e,n}__destroy_into_raw(){const e=this.ptr;return this.ptr=0,e}free(){const e=this.__destroy_into_raw();r.__wbg_searcher_free(e)}get_ptr(){return r.searcher_get_ptr(this.ptr)}}function $(e){i(e)}function I(e){return g(S.__wrap(e))}function P(e){return g(O.__wrap(e))}function C(e,n){return g(JSON.parse(d(e,n)))}function E(e,n){const _=f(n);var t=y(JSON.stringify(void 0===_?null:_),r.__wbindgen_malloc,r.__wbindgen_realloc),c=w;m()[e/4+1]=c,m()[e/4+0]=t}function R(e){const n=i(e).original;if(1==n.cnt--)return n.a=0,!0;return!1}function T(e,n){return g(f(e).fetch(f(n)))}function k(e,n,_){return g(f(e).fetch(d(n,_)))}function z(e){return f(e)instanceof Response}function F(){return A((function(e){return g(f(e).arrayBuffer())}),arguments)}function D(){return A((function(e,n){return g(new Request(d(e,n)))}),arguments)}function X(){return A((function(e,n){return g(f(e).call(f(n)))}),arguments)}function Y(e){return g(f(e))}function U(e,n){return g(new Function(d(e,n)))}function B(){return A((function(e,n,_){return g(f(e).call(f(n),f(_)))}),arguments)}function Q(e,n){try{var _={a:e,b:n},t=new Promise(((e,n)=>{const t=_.a;_.a=0;try{return function(e,n,_,t){r.wasm_bindgen__convert__closures__invoke2_mut__hb4e3db1a0f522d0a(e,n,g(_),g(t))}(t,_.b,e,n)}finally{_.a=t}}));return g(t)}finally{_.a=_.b=0}}function G(e){return g(Promise.resolve(f(e)))}function H(e,n){return g(f(e).then(f(n)))}function J(e,n,_){return g(f(e).then(f(n),f(_)))}function K(){return A((function(){return g(self.self)}),arguments)}function L(){return A((function(){return g(window.window)}),arguments)}function N(){return A((function(){return g(globalThis.globalThis)}),arguments)}function Z(){return A((function(){return g(_.g.global)}),arguments)}function M(e){return void 0===f(e)}function W(e){return g(f(e).buffer)}function V(e){return f(e).length}function ee(e){return g(new Uint8Array(f(e)))}function ne(e,n,_){f(e).set(f(n),_>>>0)}function _e(e,n){var _=y(q(f(n)),r.__wbindgen_malloc,r.__wbindgen_realloc),t=w;m()[e/4+1]=t,m()[e/4+0]=_}function te(e,n){throw new Error(d(e,n))}function re(){return g(r.memory)}function ce(e,n,_){return g(function(e,n,_,t){const c={a:e,b:n,cnt:1,dtor:_},o=(...e)=>{c.cnt++;const n=c.a;c.a=0;try{return t(n,c.b,...e)}finally{0==--c.cnt?r.__wbindgen_export_2.get(c.dtor)(n,c.b):c.a=n}};return o.original=c,o}(e,n,298,v))}}))},945:(e,n,_)=>{var t=([t])=>_.v(n,e.id,"19eb3ccc8b179130a119",{"./index_bg.js":{__wbindgen_object_drop_ref:t.ug,__wbg_searcher_new:t.cI,__wbg_query_new:t.ff,__wbindgen_json_parse:t.t$,__wbindgen_json_serialize:t.r1,__wbindgen_cb_drop:t.G6,__wbg_fetch_eb9fd115eef29d0c:t.dZ,__wbg_fetch_b90bd6bfc2ff5f95:t.I8,__wbg_instanceof_Response_d61ff4c524b8dbc4:t.mC,__wbg_arrayBuffer_b7c95af83e1e2705:t.s,__wbg_newwithstr_07dc8adf8bcc4e86:t.Ey,__wbg_call_ba36642bd901572b:t.qw,__wbindgen_object_clone_ref:t.m_,__wbg_newnoargs_9fdd8f3961dd1bee:t.UL,__wbg_call_3fc07b7d5fc9022d:t.tw,__wbg_new_c143a4f563f78c4e:t.Y4,__wbg_resolve_cae3d8f752f5db88:t.Xb,__wbg_then_c2361a9d5c9a4fcb:t.g1,__wbg_then_6c9a4bf55755f9b8:t.KA,__wbg_self_bb69a836a72ec6e9:t.tS,__wbg_window_3304fc4b414c9693:t.R$,__wbg_globalThis_e0d21cabc6630763:t.md,__wbg_global_8463719227271676:t.IF,__wbindgen_is_undefined:t.XP,__wbg_buffer_9e184d6f785de5ed:t.zP,__wbg_length_2d56cb37075fcfb1:t.uQ,__wbg_new_e8101319e4cf95fc:t.td,__wbg_set_e8ae7b27314e8b98:t.Ct,__wbindgen_debug_string:t.fY,__wbindgen_throw:t.Or,__wbindgen_memory:t.oH,__wbindgen_closure_wrapper1147:t.DP}});_.a(e,(e=>{var n=e([_(686)]);return n.then?n.then(t):t(n)}),1)}}]);