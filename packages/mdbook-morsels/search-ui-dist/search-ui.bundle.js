!function(t,e){"object"==typeof exports&&"object"==typeof module?module.exports=e():"function"==typeof define&&define.amd?define([],e):"object"==typeof exports?exports.initMorsels=e():t.initMorsels=e()}(self,(function(){return(()=>{"use strict";var t={d:(e,n)=>{for(var o in n)t.o(n,o)&&!t.o(e,o)&&Object.defineProperty(e,o,{enumerable:!0,get:n[o]})},o:(t,e)=>Object.prototype.hasOwnProperty.call(t,e)},e={};t.d(e,{default:()=>Wt});const n=class{constructor(t,e,n,o,i){this.query=t,this.searchedTerms=e,this.queryParts=n,this.getNextN=o,this.free=i}};function o(t,e,n){const{fieldStoreBlockSize:o,numStoresPerDir:i,indexingConfig:s}=n,{numDocsPerBlock:r}=s,l=Math.floor(e/o),c=Math.floor(e/r);return`${t}field_store/${Math.floor(l/i)}/${l}--${c}.json`}var i=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};const s=class{constructor(t,e){this.t=t,this.i=e,this.l=Object.create(null)}u(t,e,n){return i(this,void 0,void 0,(function*(){const i=o(t,this.t,n);try{const t=yield e.getJson(i);let o=this.t%n.fieldStoreBlockSize;const{numDocsPerBlock:s}=n.indexingConfig;s<n.fieldStoreBlockSize&&(o%=s),this.l=t[o]}catch(t){console.log(t)}}))}getFields(){return this.l.map((([t,e])=>[this.i[t].name,e]))}};var r=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};let l=0;const c=[];function a(){c.length&&c.shift()().then(a)}function u(t){return r(this,void 0,void 0,(function*(){const e=()=>r(this,void 0,void 0,(function*(){l+=1,yield t(),l-=1}));l>=2?c.push(e):(yield e(),a())}))}class f{constructor(t){this.cache=t,this.h=Object.create(null)}m(t){return r(this,void 0,void 0,(function*(){if(this.cache){let e=yield this.cache.match(t);e?this.h[t]=e.json():u((()=>r(this,void 0,void 0,(function*(){yield this.cache.add(t),e=yield this.cache.match(t),this.h[t]=e.json()}))))}else u((()=>r(this,void 0,void 0,(function*(){const e=yield fetch(t);this.h[t]=e.json()}))))}))}p(t){return r(this,void 0,void 0,(function*(){if(this.cache){(yield this.cache.match(t))||u((()=>this.cache.add(t)))}}))}getJson(t){return this.h[t]||(this.h[t]=fetch(t).then((t=>t.json()))),this.h[t]}}var d=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};let h;if(document.currentScript)h=document.currentScript.src;else{const t=document.getElementsByTagName("script");h=t.length&&t[t.length-1].src}h=h.replace(/#.*$/,"").replace(/\?.*$/,"").replace(/\/[^\/]+$/,"/");const m=class{constructor(t){this.v=t,this.isSetupDone=!1,this.g=Object.create(null),this.k=0,this.setupPromise=this._().then((()=>new Promise((e=>{const n=`const __morsWrkrUrl="${new URL(h+`search-worker-${this.cfg.langConfig.lang}.bundle.js`,document.baseURI||self.location.href)+""}";importScripts(__morsWrkrUrl);`,o=URL.createObjectURL(new Blob([n],{type:"text/javascript"}));this.R=new Worker(o);const i=this.$(`morsels:${t.url}`);this.R.onmessage=t=>{if(t.data.query){const{query:e,queryId:n,nextResults:o,searchedTerms:i,queryParts:s}=t.data;this.g[e][n].resolve({query:e,nextResults:o,searchedTerms:i,queryParts:s})}else""===t.data?(i.then((()=>this.R.postMessage(this.cfg))),URL.revokeObjectURL(o)):t.data.isSetupDone&&(this.isSetupDone=!0,e(),this.O(),this.S())},this.R.onmessageerror=t=>{console.log(t)}}))))}$(t){return d(this,void 0,void 0,(function*(){try{let e=yield caches.open(t);const n=yield e.match("/index_ver");if(n){const o=yield n.text();this.cfg.indexVer!==o&&(yield caches.delete(t),e=yield caches.open(t))}yield e.put("/index_ver",new Response(this.cfg.indexVer)),this.P=new f(e)}catch(t){this.P=new f(void 0)}}))}O(){if(!this.v.cacheAllFieldStores)return;const{fieldStoreBlockSize:t,indexingConfig:e}=this.cfg,n=Math.min(t,e.numDocsPerBlock);for(let t=0;t<this.cfg.lastDocId;t+=n)this.P.m(o(this.v.url,t,this.cfg))}S(){this.cfg.indexingConfig.plNamesToCache.forEach((t=>{const e=Math.floor(t/this.cfg.indexingConfig.numPlsPerDir),n=`${this.v.url}pl_${e}/pl_${t}.json`;this.P.p(n)}))}_(){return d(this,void 0,void 0,(function*(){if(this.cfg=yield(yield fetch(`${this.v.url}morsels_config.json`)).json(),"0.2.5"!==this.cfg.ver)throw new Error("Morsels search !== indexer version!");"cacheAllFieldStores"in this.v||(this.v.cacheAllFieldStores=!!this.cfg.cacheAllFieldStores),this.v.useQueryTermProximity=this.v.useQueryTermProximity&&this.cfg.indexingConfig.withPositions,this.cfg.searcherOptions=this.v}))}T(t,e){delete this.g[t][e],0===Object.keys(this.g[t]).length&&delete this.g[t]}getQuery(t){return d(this,void 0,void 0,(function*(){yield this.setupPromise;const e=this.k;this.k+=1,this.g[t]=this.g[t]||{},this.g[t][e]={promise:void 0,resolve:void 0},this.g[t][e].promise=new Promise((n=>{this.g[t][e].resolve=n,this.R.postMessage({query:t,queryId:e})}));const o=yield this.g[t][e].promise;return new n(t,o.searchedTerms,o.queryParts,(n=>d(this,void 0,void 0,(function*(){if(!this.g[t]||!this.g[t][e])return[];if(yield this.g[t][e].promise,this.g[t][e].promise=new Promise((o=>{this.g[t][e].resolve=o,this.R.postMessage({query:t,queryId:e,isGetNextN:!0,n})})),!this.g[t]||!this.g[t][e])return[];const o=(yield this.g[t][e].promise).nextResults.map((t=>new s(t,this.cfg.fieldInfos)));return yield Promise.all(o.map((t=>t.u(this.v.url,this.P,this.cfg)))),o}))),(()=>{this.T(t,e),this.R.postMessage({query:t,isFree:!0})}))}))}};function p(t){if("string"!=typeof t)throw new TypeError("Expected a string");return t.replace(/[|\\{}()[\]^$+*?.]/g,"\\$&").replace(/-/g,"\\x2d")}var y;function w(t,e,...n){const o=document.createElement(t);return Object.entries(e).forEach((([t,e])=>{o.setAttribute(t,e)})),n.forEach((t=>{if("string"==typeof t){const e=document.createElement("span");e.textContent=t,o.appendChild(e)}else o.appendChild(t)})),o}!function(t){t.Auto="auto",t.Dropdown="dropdown",t.Fullscreen="fullscreen",t.Target="target"}(y||(y={}));const v=w,b="data-morsels-loading-indicator";function g(){return w("div",{[b]:"true"})}function x(t){return t.startsWith("/")?new URL(new URL(window.location.href).origin+t):new URL(t)}function k(t,e){const n=e.map((e=>Array.from(t.matchAll(e))));if(!n.some((t=>t.length)))return{j:t,M:[],A:0};let o=e.map((()=>-1)),i=1e7,s=e.map((()=>0));const r=n.map((()=>0)),l=n.map((t=>!t.length)),c=n.map((()=>-1));for(;;){let t=1e7,e=1e7,a=-1,u=0;for(let o=0;o<r.length;o++){const i=n[o][r[o]];if(!i)continue;const s=i.index+i[1].length;!l[o]&&s<e&&(e=s,a=o),t=Math.min(t,s),u=Math.max(u,s),c[o]=s}if(-1===a)break;const f=u-t;if(f<i&&(i=f,o=[...c],s=r.map(((t,e)=>n[e][t]&&n[e][t][2].length+n[e][t][3].length))),r[a]+=1,r[a]>=n[a].length&&(l[a]=!0,r[a]-=1,l.every((t=>t))))break}const a=o.map(((t,e)=>({pos:t,len:s[e]}))).filter((t=>t.pos>=0)).sort(((t,e)=>t.pos-e.pos)),u=a.length;return{j:t,M:a,A:u}}function _(){return v("span",{class:"morsels-ellipsis","aria-label":"ellipses"}," ... ")}function R(t,e,n){const{highlightRender:o}=n.uiOptions.resultsRenderOpts,{j:i,M:s}=t;if(!s.some((({pos:t})=>t>=0))){if(e){const t=i.trimStart().substring(0,80);return[80===t.length?t.replace(/\w+$/,""):t,_()]}return[i]}const r=[];let l=0;for(const{pos:t,len:c}of s){const s=t+c;if(t>l+80){e&&r.push(_());const l=i.substring(t-40,t);r.push(40===l.length?l.replace(/^\w+/,""):l),r.push(o(v,n,i.substring(t,s)))}else if(t>=l)r.pop(),r.push(i.substring(l,t)),r.push(o(v,n,i.substring(t,s)));else{if(!(s>l))continue;r.pop();r[r.length-1].textContent+=i.substring(l,s)}const a=i.substring(s,s+40);r.push(40===a.length?a.replace(/\w+$/,""):a),l=s}return e&&r.push(_()),r}function $(t,e,n,o){const{maxSubMatches:i,resultsRenderOpts:s}=o.uiOptions,{bodyOnlyRender:r,headingBodyRender:l}=s;let c,a=-2,u="",f=[];for(let n=0;n<t.length;n+=1){const[o,i]=t[n];switch(o){case"headingLink":a=n,u=i;break;case"heading":c=k(i,e),c.D=n,c.C=a===c.D-1?u:"",f.push({j:"",M:[],A:-2e3,L:c,C:c.C,D:n});break;case"body":{const t=k(i,e);c?(t.L=c,t.C=c.C,t.A+=c.A):t.A-=1e3,f.push(t);break}}}f.sort(((t,e)=>0===t.A&&0===e.A?e.j.length-t.j.length:e.A-t.A));const d=[],h=Math.min(f.length,i);for(let t=0;t<h&&f[t].A===f[0].A;t+=1)d.push(f[t]);return d.map((t=>{const e=R(t,!0,o);if(t.L){const i=R(t.L,!1,o),s=i.length?i:[t.L.j],r=t.C&&`${n}#${t.C}`;return l(v,o,s,e,r)}return r(v,o,e)}))}var O=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};const S=new DOMParser;function P(t,e,n,o,i,s){return O(this,void 0,void 0,(function*(){const{loaderConfigs:r}=n.indexingConfig,l=t.getFields();let c,a,u,f=!1;for(const t of l){const[e,n]=t;switch(e){case"link":c=c||n;break;case"_relative_fp":a=a||n;break;case"title":u=u||n;break;case"h1":f||(u=n,f=!0)}if(c&&a&&u&&f)break}const{sourceFilesUrl:d,resultsRenderOpts:{addSearchedTerms:h,listItemRender:m}}=e.uiOptions,p=c||"string"==typeof d&&a&&`${d}${a}`||"";if(!u)if(a){const t=a.split("/").join(" » ").split(".");let e=t.pop().toUpperCase();e="HTML"===e?"":"PDF"===e?" (PDF)":"."+e,u=t.join(".")+e}else u=c;let y,w=p;if(h&&p){const t=x(p);t.searchParams.append(h,i),w=t.toString()}if(o)y=$(l,s,w,e);else if(p)if(p.endsWith(".html")&&r.HtmlLoader){const t=yield(yield fetch(p)).text(),n=S.parseFromString(t,"text/html"),{title:o,bodies:i}=function(t,e,n,o,i){const s=[];if(e.exclude_selectors)for(const n of e.exclude_selectors){const e=t.querySelectorAll(n);for(let t=0;t<e.length;t+=1)e[t].remove()}e.selectors=e.selectors||[];const r=e.selectors.map((t=>t.selector)).join(",");!function t(n,o){for(const t of e.selectors)if(n.matches(t.selector)){Object.entries(t.attr_map).forEach((([t,e])=>{n.attributes[t]&&s.push([e,n.attributes[t].value])})),o=t.field_name;break}if(n.querySelector(r))for(let e=0;e<n.childNodes.length;e+=1){const i=n.childNodes[e];i.nodeType===Node.ELEMENT_NODE?t(i,o):i.nodeType===Node.TEXT_NODE&&o&&(s.length&&s[s.length-1][0]===o?s[s.length-1][1]+=i.data:s.push([o,i.data]))}else o&&(s.length&&s[s.length-1][0]===o?s[s.length-1][1]+=n.textContent:s.push([o,n.textContent||""]))}(t.documentElement,void 0);let l="",c=!1;for(const t of s){const[e,n]=t;if("title"===e?l=l||n:"h1"!==e||c||(l=n,c=!0),l&&c)break}return{title:l,bodies:$(s,n,o,i)}}(n,r.HtmlLoader,s,w,e);u=o||u,y=i}else if(p.endsWith(".txt")&&r.TxtLoader){y=$([["body",yield(yield fetch(p)).text()]],s,w,e)}else{const t=x(p);if(t.pathname.endsWith(".json")&&r.JsonLoader){const n=yield(yield fetch(p)).json(),{title:o,bodies:i}=function(t,e,n,o,i){const s=[],{field_map:r,field_order:l}=e,c=Object.entries(r).find((([,t])=>"title"===t)),a=c&&c[0];for(const e of l)e!==a&&t[e]&&s.push([r[e],t[e]]);return{title:a&&t[a],bodies:$(s,n,o,i)}}(t.hash?n[t.hash.substring(1)]:n,r.JsonLoader,s,w,e);u=o||u,y=i}}else y=[];return m(v,e,i,p,u,y,l)}))}function T(t,e,n,o,i){const s=[],r=[];for(const t of i.searchedTerms){const e=t.map((t=>(r.push(t),p(t)))).sort(((t,e)=>e.length-t.length)).join("|");if("ascii"===n.langConfig.lang){const t=new RegExp(`(^|\\W|_)(${e})((?=\\W|$))`,"gi");s.push(t)}else if("latin"===n.langConfig.lang){const t=new RegExp(`(^|\\W|_)(${e})(\\w*?)(?=\\W|$)`,"gi");s.push(t)}else if("chinese"===n.langConfig.lang){const t=new RegExp(`()(${e})()`,"gi");s.push(t)}}const l=n.fieldInfos.find((t=>t.do_store&&("body"===t.name||"title"===t.name||"heading"===t.name)));return Promise.all(o.map((t=>P(t,e,n,l,JSON.stringify(r),s))))}function j(t,e,n,o,i,s){return O(this,void 0,void 0,(function*(){if(t.W)return!1;const{loadingIndicatorRender:r,termInfoRender:l,resultsPerPage:c,resultsRender:a,noResultsRender:u,mode:f}=s.uiOptions,d=r(v,s,!1,!0);o||i.appendChild(d),t.I&&t.I.disconnect();const h=document.createDocumentFragment();(o?l(v,s,e.queryParts):[]).forEach((t=>h.appendChild(t)));const m=yield e.getNextN(c);if(t.W)return!1;const p=yield a(v,s,n,m,e);if(t.W)return!1;p.length?p.forEach((t=>h.appendChild(t))):o&&h.appendChild(u(v,s));const w=h.lastElementChild;if(o?(i.innerHTML="",t.N=g(),i.append(t.N),i.append(h)):d.replaceWith(h),p.length){const o=f===y.Target?null:i;t.I=new IntersectionObserver((([o],r)=>O(this,void 0,void 0,(function*(){o.isIntersecting&&(r.unobserve(w),yield j(t,e,n,!1,i,s))}))),{root:o,rootMargin:"150px 0px"}),t.I.observe(w)}return!0}))}class E{constructor(){this.q=!0,this.F=!1,this.N=g()}}function M(t,e,n){t.setAttribute("role","combobox"),t.setAttribute("aria-expanded","true"),t.setAttribute("aria-owns",e.getAttribute("id")),e.setAttribute("role","listbox"),e.setAttribute("aria-label",n),e.setAttribute("aria-live","polite")}function A(t,e){t.setAttribute("autocomplete","off"),t.setAttribute("aria-autocomplete","list"),t.setAttribute("aria-controls",e),t.setAttribute("aria-activedescendant","morsels-list-selected")}function D(t){return t.split("-")[0]}function C(t){return t.split("-")[1]}function L(t){return["top","bottom"].includes(D(t))?"x":"y"}function W(t){return"y"===t?"height":"width"}function I(t,e,n){let{reference:o,floating:i}=t;const s=o.x+o.width/2-i.width/2,r=o.y+o.height/2-i.height/2,l=L(e),c=W(l),a=o[c]/2-i[c]/2,u="x"===l;let f;switch(D(e)){case"top":f={x:s,y:o.y-i.height};break;case"bottom":f={x:s,y:o.y+o.height};break;case"right":f={x:o.x+o.width,y:r};break;case"left":f={x:o.x-i.width,y:r};break;default:f={x:o.x,y:o.y}}switch(C(e)){case"start":f[l]-=a*(n&&u?-1:1);break;case"end":f[l]+=a*(n&&u?-1:1)}return f}function N(t){return"number"!=typeof t?function(t){return{top:0,right:0,bottom:0,left:0,...t}}(t):{top:t,right:t,bottom:t,left:t}}function q(t){return{...t,top:t.y,left:t.x,right:t.x+t.width,bottom:t.y+t.height}}async function F(t,e){var n;void 0===e&&(e={});const{x:o,y:i,platform:s,rects:r,elements:l,strategy:c}=t,{boundary:a="clippingAncestors",rootBoundary:u="viewport",elementContext:f="floating",altBoundary:d=!1,padding:h=0}=e,m=N(h),p=l[d?"floating"===f?"reference":"floating":f],y=q(await s.getClippingRect({element:null==(n=await(null==s.isElement?void 0:s.isElement(p)))||n?p:p.contextElement||await(null==s.getDocumentElement?void 0:s.getDocumentElement(l.floating)),boundary:a,rootBoundary:u,strategy:c})),w=q(s.convertOffsetParentRelativeRectToViewportRelativeRect?await s.convertOffsetParentRelativeRectToViewportRelativeRect({rect:"floating"===f?{...r.floating,x:o,y:i}:r.reference,offsetParent:await(null==s.getOffsetParent?void 0:s.getOffsetParent(l.floating)),strategy:c}):r[f]);return{top:y.top-w.top+m.top,bottom:w.bottom-y.bottom+m.bottom,left:y.left-w.left+m.left,right:w.right-y.right+m.right}}const U=Math.min,B=Math.max;function H(t,e,n){return B(t,U(e,n))}const z=t=>({name:"arrow",options:t,async fn(e){const{element:n,padding:o=0}=null!=t?t:{},{x:i,y:s,placement:r,rects:l,platform:c}=e;if(null==n)return{};const a=N(o),u={x:i,y:s},f=L(r),d=W(f),h=await c.getDimensions(n),m="y"===f?"top":"left",p="y"===f?"bottom":"right",y=l.reference[d]+l.reference[f]-u[f]-l.floating[d],w=u[f]-l.reference[f],v=await(null==c.getOffsetParent?void 0:c.getOffsetParent(n));let b=v?"y"===f?v.clientHeight||0:v.clientWidth||0:0;0===b&&(b=l.floating[d]);const g=y/2-w/2,x=a[m],k=b-h[d]-a[p],_=b/2-h[d]/2+g,R=H(x,_,k);return{data:{[f]:R,centerOffset:_-R}}}}),J={left:"right",right:"left",bottom:"top",top:"bottom"};function Q(t){return t.replace(/left|right|bottom|top/g,(t=>J[t]))}function G(t,e,n){void 0===n&&(n=!1);const o=C(t),i=L(t),s=W(i);let r="x"===i?o===(n?"end":"start")?"right":"left":"start"===o?"bottom":"top";return e.reference[s]>e.floating[s]&&(r=Q(r)),{main:r,cross:Q(r)}}const V={start:"end",end:"start"};function X(t){return t.replace(/start|end/g,(t=>V[t]))}const Y=["top","right","bottom","left"],K=(Y.reduce(((t,e)=>t.concat(e,e+"-start",e+"-end")),[]),function(t){return void 0===t&&(t={}),{name:"flip",options:t,async fn(e){var n;const{placement:o,middlewareData:i,rects:s,initialPlacement:r,platform:l,elements:c}=e,{mainAxis:a=!0,crossAxis:u=!0,fallbackPlacements:f,fallbackStrategy:d="bestFit",flipAlignment:h=!0,...m}=t,p=D(o),y=f||(p!==r&&h?function(t){const e=Q(t);return[X(t),e,X(e)]}(r):[Q(r)]),w=[r,...y],v=await F(e,m),b=[];let g=(null==(n=i.flip)?void 0:n.overflows)||[];if(a&&b.push(v[p]),u){const{main:t,cross:e}=G(o,s,await(null==l.isRTL?void 0:l.isRTL(c.floating)));b.push(v[t],v[e])}if(g=[...g,{placement:o,overflows:b}],!b.every((t=>t<=0))){var x,k;const t=(null!=(x=null==(k=i.flip)?void 0:k.index)?x:0)+1,e=w[t];if(e)return{data:{index:t,overflows:g},reset:{placement:e}};let n="bottom";switch(d){case"bestFit":{var _;const t=null==(_=g.map((t=>[t,t.overflows.filter((t=>t>0)).reduce(((t,e)=>t+e),0)])).sort(((t,e)=>t[1]-e[1]))[0])?void 0:_[0].placement;t&&(n=t);break}case"initialPlacement":n=r}if(o!==n)return{reset:{placement:n}}}return{}}}});const Z=function(t){return void 0===t&&(t={}),{name:"size",options:t,async fn(e){const{placement:n,rects:o,platform:i,elements:s}=e,{apply:r,...l}=t,c=await F(e,l),a=D(n),u=C(n);let f,d;"top"===a||"bottom"===a?(f=a,d=u===(await(null==i.isRTL?void 0:i.isRTL(s.floating))?"start":"end")?"left":"right"):(d=a,f="end"===u?"top":"bottom");const h=B(c.left,0),m=B(c.right,0),p=B(c.top,0),y=B(c.bottom,0),w={availableHeight:o.floating.height-(["left","right"].includes(n)?2*(0!==p||0!==y?p+y:B(c.top,c.bottom)):c[f]),availableWidth:o.floating.width-(["top","bottom"].includes(n)?2*(0!==h||0!==m?h+m:B(c.left,c.right)):c[d])},v=await i.getDimensions(s.floating);null==r||r({...e,...w});const b=await i.getDimensions(s.floating);return v.width!==b.width||v.height!==b.height?{reset:{rects:!0}}:{}}}};function tt(t){return t&&t.document&&t.location&&t.alert&&t.setInterval}function et(t){if(null==t)return window;if(!tt(t)){const e=t.ownerDocument;return e&&e.defaultView||window}return t}function nt(t){return et(t).getComputedStyle(t)}function ot(t){return tt(t)?"":t?(t.nodeName||"").toLowerCase():""}function it(){const t=navigator.userAgentData;return null!=t&&t.brands?t.brands.map((t=>t.brand+"/"+t.version)).join(" "):navigator.userAgent}function st(t){return t instanceof et(t).HTMLElement}function rt(t){return t instanceof et(t).Element}function lt(t){return"undefined"!=typeof ShadowRoot&&(t instanceof et(t).ShadowRoot||t instanceof ShadowRoot)}function ct(t){const{overflow:e,overflowX:n,overflowY:o}=nt(t);return/auto|scroll|overlay|hidden/.test(e+o+n)}function at(t){return["table","td","th"].includes(ot(t))}function ut(t){const e=/firefox/i.test(it()),n=nt(t);return"none"!==n.transform||"none"!==n.perspective||"paint"===n.contain||["transform","perspective"].includes(n.willChange)||e&&"filter"===n.willChange||e&&!!n.filter&&"none"!==n.filter}function ft(){return!/^((?!chrome|android).)*safari/i.test(it())}const dt=Math.min,ht=Math.max,mt=Math.round;function pt(t,e,n){var o,i,s,r;void 0===e&&(e=!1),void 0===n&&(n=!1);const l=t.getBoundingClientRect();let c=1,a=1;e&&st(t)&&(c=t.offsetWidth>0&&mt(l.width)/t.offsetWidth||1,a=t.offsetHeight>0&&mt(l.height)/t.offsetHeight||1);const u=rt(t)?et(t):window,f=!ft()&&n,d=(l.left+(f&&null!=(o=null==(i=u.visualViewport)?void 0:i.offsetLeft)?o:0))/c,h=(l.top+(f&&null!=(s=null==(r=u.visualViewport)?void 0:r.offsetTop)?s:0))/a,m=l.width/c,p=l.height/a;return{width:m,height:p,top:h,right:d+m,bottom:h+p,left:d,x:d,y:h}}function yt(t){return(e=t,(e instanceof et(e).Node?t.ownerDocument:t.document)||window.document).documentElement;var e}function wt(t){return rt(t)?{scrollLeft:t.scrollLeft,scrollTop:t.scrollTop}:{scrollLeft:t.pageXOffset,scrollTop:t.pageYOffset}}function vt(t){return pt(yt(t)).left+wt(t).scrollLeft}function bt(t,e,n){const o=st(e),i=yt(e),s=pt(t,o&&function(t){const e=pt(t);return mt(e.width)!==t.offsetWidth||mt(e.height)!==t.offsetHeight}(e),"fixed"===n);let r={scrollLeft:0,scrollTop:0};const l={x:0,y:0};if(o||!o&&"fixed"!==n)if(("body"!==ot(e)||ct(i))&&(r=wt(e)),st(e)){const t=pt(e,!0);l.x=t.x+e.clientLeft,l.y=t.y+e.clientTop}else i&&(l.x=vt(i));return{x:s.left+r.scrollLeft-l.x,y:s.top+r.scrollTop-l.y,width:s.width,height:s.height}}function gt(t){return"html"===ot(t)?t:t.assignedSlot||t.parentNode||(lt(t)?t.host:null)||yt(t)}function xt(t){return st(t)&&"fixed"!==getComputedStyle(t).position?t.offsetParent:null}function kt(t){const e=et(t);let n=xt(t);for(;n&&at(n)&&"static"===getComputedStyle(n).position;)n=xt(n);return n&&("html"===ot(n)||"body"===ot(n)&&"static"===getComputedStyle(n).position&&!ut(n))?e:n||function(t){let e=gt(t);for(lt(e)&&(e=e.host);st(e)&&!["html","body"].includes(ot(e));){if(ut(e))return e;e=e.parentNode}return null}(t)||e}function _t(t){if(st(t))return{width:t.offsetWidth,height:t.offsetHeight};const e=pt(t);return{width:e.width,height:e.height}}function Rt(t){const e=gt(t);return["html","body","#document"].includes(ot(e))?t.ownerDocument.body:st(e)&&ct(e)?e:Rt(e)}function $t(t,e){var n;void 0===e&&(e=[]);const o=Rt(t),i=o===(null==(n=t.ownerDocument)?void 0:n.body),s=et(o),r=i?[s].concat(s.visualViewport||[],ct(o)?o:[]):o,l=e.concat(r);return i?l:l.concat($t(r))}function Ot(t,e,n){return"viewport"===e?q(function(t,e){const n=et(t),o=yt(t),i=n.visualViewport;let s=o.clientWidth,r=o.clientHeight,l=0,c=0;if(i){s=i.width,r=i.height;const t=ft();(t||!t&&"fixed"===e)&&(l=i.offsetLeft,c=i.offsetTop)}return{width:s,height:r,x:l,y:c}}(t,n)):rt(e)?function(t,e){const n=pt(t,!1,"fixed"===e),o=n.top+t.clientTop,i=n.left+t.clientLeft;return{top:o,left:i,x:i,y:o,right:i+t.clientWidth,bottom:o+t.clientHeight,width:t.clientWidth,height:t.clientHeight}}(e,n):q(function(t){var e;const n=yt(t),o=wt(t),i=null==(e=t.ownerDocument)?void 0:e.body,s=ht(n.scrollWidth,n.clientWidth,i?i.scrollWidth:0,i?i.clientWidth:0),r=ht(n.scrollHeight,n.clientHeight,i?i.scrollHeight:0,i?i.clientHeight:0);let l=-o.scrollLeft+vt(t);const c=-o.scrollTop;return"rtl"===nt(i||n).direction&&(l+=ht(n.clientWidth,i?i.clientWidth:0)-s),{width:s,height:r,x:l,y:c}}(yt(t)))}function St(t){const e=$t(t),n=["absolute","fixed"].includes(nt(t).position)&&st(t)?kt(t):t;return rt(n)?e.filter((t=>rt(t)&&function(t,e){const n=null==e||null==e.getRootNode?void 0:e.getRootNode();if(null!=t&&t.contains(e))return!0;if(n&&lt(n)){let n=e;do{if(n&&t===n)return!0;n=n.parentNode||n.host}while(n)}return!1}(t,n)&&"body"!==ot(t))):[]}const Pt={getClippingRect:function(t){let{element:e,boundary:n,rootBoundary:o,strategy:i}=t;const s=[..."clippingAncestors"===n?St(e):[].concat(n),o],r=s[0],l=s.reduce(((t,n)=>{const o=Ot(e,n,i);return t.top=ht(o.top,t.top),t.right=dt(o.right,t.right),t.bottom=dt(o.bottom,t.bottom),t.left=ht(o.left,t.left),t}),Ot(e,r,i));return{width:l.right-l.left,height:l.bottom-l.top,x:l.left,y:l.top}},convertOffsetParentRelativeRectToViewportRelativeRect:function(t){let{rect:e,offsetParent:n,strategy:o}=t;const i=st(n),s=yt(n);if(n===s)return e;let r={scrollLeft:0,scrollTop:0};const l={x:0,y:0};if((i||!i&&"fixed"!==o)&&(("body"!==ot(n)||ct(s))&&(r=wt(n)),st(n))){const t=pt(n,!0);l.x=t.x+n.clientLeft,l.y=t.y+n.clientTop}return{...e,x:e.x-r.scrollLeft+l.x,y:e.y-r.scrollTop+l.y}},isElement:rt,getDimensions:_t,getOffsetParent:kt,getDocumentElement:yt,getElementRects:t=>{let{reference:e,floating:n,strategy:o}=t;return{reference:bt(e,kt(n),o),floating:{..._t(n),x:0,y:0}}},getClientRects:t=>Array.from(t.getClientRects()),isRTL:t=>"rtl"===nt(t).direction};const Tt=(t,e,n)=>(async(t,e,n)=>{const{placement:o="bottom",strategy:i="absolute",middleware:s=[],platform:r}=n,l=await(null==r.isRTL?void 0:r.isRTL(e));let c=await r.getElementRects({reference:t,floating:e,strategy:i}),{x:a,y:u}=I(c,o,l),f=o,d={};for(let n=0;n<s.length;n++){const{name:h,fn:m}=s[n],{x:p,y,data:w,reset:v}=await m({x:a,y:u,initialPlacement:o,placement:f,strategy:i,middlewareData:d,rects:c,platform:r,elements:{reference:t,floating:e}});a=null!=p?p:a,u=null!=y?y:u,d={...d,[h]:{...d[h],...w}},v&&("object"==typeof v&&(v.placement&&(f=v.placement),v.rects&&(c=!0===v.rects?await r.getElementRects({reference:t,floating:e,strategy:i}):v.rects),({x:a,y:u}=I(c,f,l))),n=-1)}return{x:a,y:u,placement:f,strategy:i,middlewareData:d}})(t,e,{platform:Pt,...n});function jt(t,e){if(!1===t.tip)return;function n(t){return v("code",{},t)}function o(...t){return v("tr",{class:"morsels-tip-item"},...t.map((t=>v("td",{},v("div",{},t)))))}const i=v("tbody",{},o("Require all terms to match",n("weather AND forecast AND sunny")),o("Flip search results",n("NOT rainy")),o("Group terms together",n("forecast AND (sunny warm)")),o("Match specific areas",v("ul",{},v("li",{},n("title:forecast")),v("li",{},n("heading:sunny")),v("li",{},n("body:(rainy gloomy)"))))),s=v("table",{class:"morsels-tip-table"},v("thead",{class:"morsels-tip-table-header"},v("tr",{},v("th",{scope:"col"},"Tip"),v("th",{},"Example"))),i),r=v("div",{class:"morsels-tip-popup-root"},v("div",{class:"morsels-tip-popup"},v("div",{class:"morsels-tip-popup-title"},"🔎 Didn't find what you needed?"),s),v("div",{class:"morsels-tip-popup-separator"}));function l(){Object.assign(r.style,{left:"calc(var(--morsels-tip-icon-size) - 150px)",top:"-160px"}),r.classList.remove("shown")}l();const c=v("div",{class:"morsels-tip-root",tabindex:"0"},v("span",{class:"morsels-tip-icon"},"?"),r);function a(){Tt(c,r,{placement:"top-end",middleware:[K({crossAxis:!1,flipAlignment:!1,padding:10})]}).then((({x:t,y:e})=>{Object.assign(r.style,{left:`${t}px`,top:`${e}px`}),r.classList.add("shown")}))}return c.onmouseover=a,c.onfocus=a,c.onmouseleave=l,c.onblur=l,e.setupPromise.then((()=>{e.cfg.indexingConfig.withPositions&&i.prepend(o("Search for phrases",n('"for tomorrow"')))})),c}function Et(t,e,n){t.setAttribute("autocomplete","off"),t.setAttribute("readonly",""),t.setAttribute("role","button"),t.setAttribute("aria-label",n),e&&t.setAttribute("placeholder",e),t.classList.add("morsels-button-input")}var Mt=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};let At=!1;function Dt(t){return t.mode===y.Auto&&!At||t.mode===y.Dropdown}class Ct{constructor(){this.U=!1,this.B=!1}H(t,e,n,o){const{uiOptions:i}=o,s=new E;function r(r){var l;return Mt(this,void 0,void 0,(function*(){s.F=!0;const c=i.loadingIndicatorRender(v,o,!1,s.q);s.N.replaceWith(c),s.N=c;try{null===(l=s.currQuery)||void 0===l||l.free(),s.currQuery=yield n.getQuery(r);(yield j(s,s.currQuery,n.cfg,!0,e,o))&&(s.q=!1),t.scrollTo({top:0}),e.scrollTo({top:0})}catch(t){throw console.error(t),e.innerHTML="",e.appendChild(i.errorRender(v,o)),t}finally{if(s.W){const t=s.W;s.W=void 0,yield t()}else s.F=!1}}))}n.setupPromise.then((()=>{s.W&&(s.W(),s.W=void 0)}));let l=-1;return t=>{const c=i.preprocessQuery(t.target.value);if(clearTimeout(l),c.length)l=setTimeout((()=>{var t;if(s.q&&!(null===(t=e.firstElementChild)||void 0===t?void 0:t.getAttribute(b))){e.innerHTML="";const t=i.loadingIndicatorRender(v,o,!n.isSetupDone,!0);s.N=t,e.appendChild(t),Dt(i)&&this.J()}s.F||!n.isSetupDone?s.W=()=>r(c):r(c)}),i.inputDebounce);else{const t=()=>{e.innerHTML="",Dt(i)?this.G():i.mode!==y.Target&&e.appendChild(i.fsBlankRender(v,o)),s.F=!1,s.q=!0};s.F?s.W=t:t()}}}}const Lt={};const Wt=function(t){const e=t.isMobileDevice||(()=>window.matchMedia("only screen and (max-width: 1024px)").matches);At=e(),function(t){t.searcherOptions=t.searcherOptions||{};const{searcherOptions:e}=t;if(!("url"in e))throw new Error("Mandatory url parameter not specified");e.url.endsWith("/")||(e.url+="/"),e.url.startsWith("/")&&(e.url=window.location.origin+e.url),"numberOfExpandedTerms"in e||(e.numberOfExpandedTerms=3),"useQueryTermProximity"in e||(e.useQueryTermProximity=!0),"plLazyCacheThreshold"in e||(e.plLazyCacheThreshold=0),"resultLimit"in e||(e.resultLimit=null),t.uiOptions=t.uiOptions||{};const{uiOptions:n}=t;if(n.sourceFilesUrl&&!n.sourceFilesUrl.endsWith("/")&&(n.sourceFilesUrl+="/"),n.mode=n.mode||y.Auto,n.mode===y.Target&&("string"==typeof n.target&&(n.target=document.getElementById(n.target)),!n.target))throw new Error("'target' mode specified but no valid target option specified");if("input"in n&&"string"!=typeof n.input||(n.input=document.getElementById(n.input||"morsels-search")),[y.Dropdown,y.Target].includes(n.mode)&&!n.input)throw new Error("'dropdown' or 'target' mode specified but no input element found");"inputDebounce"in n||(n.inputDebounce=100),n.preprocessQuery=n.preprocessQuery||(t=>t),n.dropdownAlignment=n.dropdownAlignment||"bottom-end","string"==typeof n.fsContainer&&(n.fsContainer=document.getElementById(n.fsContainer)),n.fsContainer=n.fsContainer||document.getElementsByTagName("body")[0],n.resultsPerPage=n.resultsPerPage||8,n.maxSubMatches=n.maxSubMatches||2,n.label=n.label||"Search this site",n.resultsLabel=n.resultsLabel||"Site results",n.fsInputLabel=n.fsInputLabel||"Search",n.fsPlaceholder=n.fsPlaceholder||"Search this site...",n.fsCloseText=n.fsCloseText||"Close",n.errorRender=n.errorRender||(t=>t("div",{class:"morsels-error"},"Oops! Something went wrong... 🙁")),n.noResultsRender=n.noResultsRender||(t=>t("div",{class:"morsels-no-results"},"No results found")),n.fsBlankRender=n.fsBlankRender||(t=>t("div",{class:"morsels-fs-blank"},"Start Searching Above!")),n.loadingIndicatorRender||(n.loadingIndicatorRender=(t,e,n,o)=>{const i=t("span",{class:"morsels-loading-indicator"});if(n){const e=t("div",{class:"morsels-initialising-text"},"... Initialising ...");return t("div",{class:"morsels-initialising"},e,i)}return o||i.classList.add("morsels-loading-indicator-subsequent"),i});const o=n.loadingIndicatorRender;n.loadingIndicatorRender=(...t)=>{const e=o(...t);return e.setAttribute(b,"true"),e},n.termInfoRender=n.termInfoRender||(()=>[]),n.resultsRender=n.resultsRender||T,n.resultsRenderOpts=n.resultsRenderOpts||{};const{resultsRenderOpts:i}=n;i.listItemRender=i.listItemRender||((t,e,n,o,s,r)=>{const l=t("a",{class:"morsels-link"},t("div",{class:"morsels-title"},s),...r);if(o){let t=o;const{addSearchedTerms:e}=i;if(e){const i=x(o);i.searchParams.append(e,n),t=i.toString()}l.setAttribute("href",t)}return t("li",{class:"morsels-list-item",role:"option"},l)}),i.headingBodyRender=i.headingBodyRender||((t,e,n,o,i)=>{const s=t("a",{class:"morsels-heading-body"},t("div",{class:"morsels-heading"},...n),t("div",{class:"morsels-bodies"},t("div",{class:"morsels-body"},...o)));return i&&s.setAttribute("href",i),s}),i.bodyOnlyRender=i.bodyOnlyRender||((t,e,n)=>t("div",{class:"morsels-body"},...n)),i.highlightRender=i.highlightRender||((t,e,n)=>t("span",{class:"morsels-highlight"},n)),t.otherOptions=t.otherOptions||{}}(t);const{uiOptions:n,searcherOptions:o}=t,{input:i,mode:s,dropdownAlignment:r,label:l,fsInputButtonText:c,fsInputLabel:a,target:u}=n,{url:f}=o;Lt[f]||(Lt[f]=new m(t.searcherOptions));const d=Lt[f],h=new Ct,[p,w,g,k,_]=function(t,e,n){const{uiOptions:o}=t,i=v("input",{class:"morsels-fs-input",type:"search",placeholder:o.fsPlaceholder,"aria-labelledby":"morsels-fs-label",enterkeyhint:"search"});A(i,"morsels-fs-list");const s=v("button",{class:"morsels-input-close-fs"},o.fsCloseText),r=v("ul",{id:"morsels-fs-list",class:"morsels-list","aria-labelledby":"morsels-fs-label"}),l=v("div",{class:"morsels-root morsels-fs-root"},v("form",{class:"morsels-fs-input-button-wrapper"},v("label",{id:"morsels-fs-label",for:"morsels-fs-input",style:"display: none"},o.label),i,s),jt(o,e),r);l.onclick=t=>t.stopPropagation(),l.onmousedown=t=>t.stopPropagation(),M(l,r,o.label);const c=v("div",{class:"morsels-fs-backdrop"},l);function a(t){n(t),c.remove()}return c.onmousedown=()=>a(!1),c.onkeyup=t=>{"Escape"===t.code&&(t.stopPropagation(),a(!0))},s.onclick=t=>{t.preventDefault(),a(""===t.pointerType)},[c,r,i,function(){o.fsContainer.appendChild(c);const t=c.querySelector("input.morsels-fs-input");t&&t.focus();const e=r.querySelector(".focus");e&&r.scrollTo({top:e.offsetTop-r.offsetTop-30})},a]}(t,d,(t=>{t&&i&&i.focus(),h.B=!1}));function R(){h.B||(k(),h.B=!0)}function $(){_(!1)}function O(){function t(){Dt(n)||R()}i.addEventListener("click",t),i.addEventListener("keydown",(e=>{"Enter"===e.key&&t()}))}let S;if(g.addEventListener("input",h.H(p,w,d,t)),w.appendChild(n.fsBlankRender(v,t)),!i||s!==y.Auto&&s!==y.Dropdown){if(i&&s===y.Fullscreen)Et(i,c,a),O();else if(i&&s===y.Target){i.addEventListener("input",h.H(u,u,d,t));let e=u.getAttribute("id");e||(u.setAttribute("id","morsels-target-list"),e="morsels-target-list"),A(i,e),M(i,u,n.label)}}else{const o=i.getAttribute("placeholder")||"",u=i.parentElement,f=u.childNodes;let m=0;for(;m<f.length&&f[m]!==i;m+=1);i.remove();const[p,w]=function(t,e,n,o){const i=v("ul",{id:"morsels-dropdown-list",class:"morsels-list",tabindex:"-1"}),s=v("div",{class:"morsels-inner-root",style:"display: none;"},v("div",{class:"morsels-input-dropdown-separator"}),jt(t,e),i),r=v("div",{class:"morsels-root"},n,s);return s.onkeyup=t=>{"Escape"===t.code&&(t.stopPropagation(),o())},[r,i]}(n,d,i,(()=>{h.G()}));function b(){s!==y.Dropdown&&(At=e())?(h.G(),function(t,e,n,o,i){t.removeAttribute("role"),t.removeAttribute("aria-expanded"),t.removeAttribute("aria-owns"),e.removeAttribute("role"),e.removeAttribute("aria-label"),e.removeAttribute("aria-live"),n.removeAttribute("aria-autocomplete"),n.removeAttribute("aria-controls"),n.removeAttribute("aria-activedescendant"),Et(n,i,o)}(p,S,i,a,c)):($(),h.G(),document.activeElement===i&&h.J(),function(t,e,n,o,i){!function(t,e){t.removeAttribute("readonly"),t.removeAttribute("role"),t.removeAttribute("aria-label"),t.setAttribute("placeholder",e),t.classList.remove("morsels-button-input")}(t,i),A(t,"morsels-dropdown-list"),M(e,n,o)}(i,p,S,l,o))}let g;S=w,m<f.length?u.insertBefore(p,f[m]):u.appendChild(p),h.J=()=>{!function(t,e,n){if(e.childElementCount){const o=t.children[1],i=o.firstElementChild;o.style.display="block",Tt(t,o,{placement:n,middleware:[K({padding:10,mainAxis:!1}),Z({apply({availableWidth:t,availableHeight:n}){Object.assign(e.style,{maxWidth:`min(${t}px, var(--morsels-dropdown-max-width))`,maxHeight:`min(${n}px, var(--morsels-dropdown-max-height))`})},padding:10}),z({element:i})]}).then((({x:t,y:e,middlewareData:n})=>{Object.assign(o.style,{left:`${t}px`,top:`${e}px`});const{x:s}=n.arrow;Object.assign(i.style,{left:null!=s?`${s}px`:""})}))}}(p,S,r),h.U=!0},h.G=()=>{p.children[1].style.display="none",h.U=!1},i.addEventListener("input",h.H(p,S,d,t)),b(),window.addEventListener("resize",(()=>{clearTimeout(g),g=setTimeout(b,10)})),p.addEventListener("focusout",(()=>{Dt(n)&&setTimeout((()=>{let t=document.activeElement;for(;t;)if(t=t.parentElement,t===p)return;h.G()}),100)})),i.addEventListener("focus",(()=>Dt(n)&&h.J())),O()}function P(t){if(!["ArrowDown","ArrowUp","Home","End","Enter"].includes(t.key))return;let e,o=t=>{const n=t.offsetTop-e.offsetTop-e.clientHeight/2+t.clientHeight/2;e.scrollTo({top:n})};if(Dt(n)){if(!h.U)return;e=S}else if(s===y.Target)e=u,o=t=>{t.scrollIntoView({block:"center"})};else{if(!h.B)return;e=w}const i=e.querySelector(".focus");function r(t){i&&(i.classList.remove("focus"),i.removeAttribute("aria-selected"),i.removeAttribute("id")),t.classList.add("focus"),t.setAttribute("aria-selected","true"),t.setAttribute("id","morsels-list-selected"),o(t)}function l(t,e){t&&!t.getAttribute(b)?r(t):e&&!e.getAttribute(b)&&r(e)}const c=e.firstElementChild,a=e.lastElementChild;if("ArrowDown"===t.key)i?l(i.nextElementSibling,null):l(c,null==c?void 0:c.nextElementSibling);else if("ArrowUp"===t.key)i&&l(i.previousElementSibling,null);else if("Home"===t.key)l(c,null==c?void 0:c.nextElementSibling);else if("End"===t.key)l(a,null==a?void 0:a.previousElementSibling);else if("Enter"===t.key&&i){const t=i.querySelector("a[href]");t&&(window.location.href=t.getAttribute("href"))}t.preventDefault()}return null==i||i.addEventListener("keydown",P),g.addEventListener("keydown",P),{showFullscreen:R,hideFullscreen:$}};return e=e.default})()}));