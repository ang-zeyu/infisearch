!function(t,e){"object"==typeof exports&&"object"==typeof module?module.exports=e():"function"==typeof define&&define.amd?define([],e):"object"==typeof exports?exports.initMorsels=e():t.initMorsels=e()}(self,(function(){return(()=>{"use strict";var t={d:(e,n)=>{for(var o in n)t.o(n,o)&&!t.o(e,o)&&Object.defineProperty(e,o,{enumerable:!0,get:n[o]})},o:(t,e)=>Object.prototype.hasOwnProperty.call(t,e)},e={};t.d(e,{default:()=>Ut});const n=class{constructor(t,e,n,o,i){this.query=t,this.searchedTerms=e,this.queryParts=n,this.getNextN=o,this.free=i}};function o(t,e,n){const{fieldStoreBlockSize:o,numStoresPerDir:i,indexingConfig:s}=n,{numDocsPerBlock:r}=s,l=Math.floor(e/o),c=Math.floor(e/r);return`${t}field_store/${Math.floor(l/i)}/${l}--${c}.json`}var i=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};const s=class{constructor(t,e,n){this.t=t,this.i=e,this.l=n,this.u=Object.create(null)}h(t,e,n){return i(this,void 0,void 0,(function*(){const i=o(t,this.t,n);try{const t=yield e.getJson(i);let o=this.t%n.fieldStoreBlockSize;const{numDocsPerBlock:s}=n.indexingConfig;s<n.fieldStoreBlockSize&&(o%=s),this.u=t[o]}catch(t){console.log(t)}}))}getFields(){return this.u.map((([t,e])=>[this.l[t].name,e]))}};var r=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};let l=0;const c=[];function a(){c.length&&c.shift()().then(a)}function u(t){return r(this,void 0,void 0,(function*(){const e=()=>r(this,void 0,void 0,(function*(){l+=1,yield t(),l-=1}));l>=2?c.push(e):(yield e(),a())}))}class f{constructor(t){this.cache=t,this.m=Object.create(null)}p(t){return r(this,void 0,void 0,(function*(){let e;this.cache?(e=yield this.cache.match(t),e||u((()=>r(this,void 0,void 0,(function*(){yield this.cache.add(t),e=yield this.cache.match(t),this.m[t]=e.json()}))))):u((()=>r(this,void 0,void 0,(function*(){e=yield fetch(t),this.m[t]=e.json()}))))}))}v(t){return r(this,void 0,void 0,(function*(){if(this.cache){(yield this.cache.match(t))||u((()=>this.cache.add(t)))}}))}getJson(t){return this.m[t]||(this.m[t]=fetch(t).then((t=>t.json()))),this.m[t]}}var d=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};let h;if(document.currentScript)h=document.currentScript.src;else{const t=document.getElementsByTagName("script");h=t.length&&t[t.length-1].src}h=h.replace(/#.*$/,"").replace(/\?.*$/,"").replace(/\/[^\/]+$/,"/");const m=class{constructor(t){this.g=t,this.isSetupDone=!1,this.k=Object.create(null),this._=0,this.setupPromise=this.$().then((()=>new Promise((e=>{const n=`const __morsWrkrUrl="${new URL(h+`search-worker-${this.cfg.langConfig.lang}.bundle.js`,document.baseURI||self.location.href)+""}";importScripts(__morsWrkrUrl);`,o=URL.createObjectURL(new Blob([n],{type:"text/javascript"}));this.O=new Worker(o);const i=this.S(`morsels:${t.url}`);this.O.onmessage=t=>{if(t.data.query){const{query:e,queryId:n,nextResults:o,searchedTerms:i,queryParts:s}=t.data;this.k[e][n].resolve({query:e,nextResults:o,searchedTerms:i,queryParts:s})}else""===t.data?(i.then((()=>this.O.postMessage(this.cfg))),URL.revokeObjectURL(o)):t.data.isSetupDone&&(this.isSetupDone=!0,e(),this.R(),this.P())},this.O.onmessageerror=t=>{console.log(t)}}))))}S(t){return d(this,void 0,void 0,(function*(){try{let e=yield caches.open(t);const n=yield e.match("/index_ver");if(n){const o=yield n.text();this.cfg.indexVer!==o&&(yield caches.delete(t),e=yield caches.open(t))}yield e.put("/index_ver",new Response(this.cfg.indexVer)),this.j=new f(e)}catch(t){this.j=new f(void 0)}}))}R(){if(!this.g.cacheAllFieldStores)return;const{fieldStoreBlockSize:t,indexingConfig:e}=this.cfg,n=Math.min(t,e.numDocsPerBlock);for(let t=0;t<this.cfg.lastDocId;t+=n)this.j.p(o(this.g.url,t,this.cfg))}P(){this.cfg.indexingConfig.plNamesToCache.forEach((t=>{const e=Math.floor(t/this.cfg.indexingConfig.numPlsPerDir),n=`${this.g.url}pl_${e}/pl_${t}.json`;this.j.v(n)}))}$(){return d(this,void 0,void 0,(function*(){if(this.cfg=yield(yield fetch(`${this.g.url}morsels_config.json`)).json(),"0.1.1"!==this.cfg.ver)throw new Error("Morsels search !== indexer version!");"cacheAllFieldStores"in this.g||(this.g.cacheAllFieldStores=!!this.cfg.cacheAllFieldStores),this.g.useQueryTermProximity=this.g.useQueryTermProximity&&this.cfg.indexingConfig.withPositions,this.cfg.searcherOptions=this.g}))}T(t,e){delete this.k[t][e],0===Object.keys(this.k[t]).length&&delete this.k[t]}getQuery(t){return d(this,void 0,void 0,(function*(){yield this.setupPromise;const e=this._;this._+=1,this.k[t]=this.k[t]||{},this.k[t][e]={promise:void 0,resolve:void 0},this.k[t][e].promise=new Promise((n=>{this.k[t][e].resolve=n,this.O.postMessage({query:t,queryId:e})}));const o=yield this.k[t][e].promise;return new n(t,o.searchedTerms,o.queryParts,(n=>d(this,void 0,void 0,(function*(){if(!this.k[t]||!this.k[t][e])return[];if(yield this.k[t][e].promise,this.k[t][e].promise=new Promise((o=>{this.k[t][e].resolve=o,this.O.postMessage({query:t,queryId:e,isGetNextN:!0,n})})),!this.k[t]||!this.k[t][e])return[];const o=(yield this.k[t][e].promise).nextResults.map((([t,e])=>new s(t,e,this.cfg.fieldInfos)));return yield Promise.all(o.map((t=>t.h(this.g.url,this.j,this.cfg)))),o}))),(()=>{this.T(t,e),this.O.postMessage({query:t,isFree:!0})}))}))}};function p(t){if("string"!=typeof t)throw new TypeError("Expected a string");return t.replace(/[|\\{}()[\]^$+*?.]/g,"\\$&").replace(/-/g,"\\x2d")}function y(t,e,...n){const o=document.createElement(t);return Object.entries(e).forEach((([t,e])=>{o.setAttribute(t,e)})),n.forEach((t=>{if("string"==typeof t){const e=document.createElement("span");e.textContent=t,o.appendChild(e)}else o.appendChild(t)})),o}const w=y,v="data-morsels-loading-indicator";function g(){return y("div",{[v]:"true"})}function b(t){return t.startsWith("/")?new URL(new URL(window.location.href).origin+t):new URL(t)}function x(t,e){const n=e.map((e=>Array.from(t.matchAll(e))));if(!n.some((t=>t.length)))return{A:t,M:[],D:0};let o=e.map((()=>-1)),i=1e7,s=e.map((()=>0));const r=n.map((()=>0)),l=n.map((t=>!t.length)),c=n.map((()=>-1));for(;;){let t=1e7,e=1e7,a=-1,u=0;for(let o=0;o<r.length;o++){const i=n[o][r[o]];if(!i)continue;const s=i.index+i[1].length;!l[o]&&s<e&&(e=s,a=o),t=Math.min(t,s),u=Math.max(u,s),c[o]=s}if(-1===a)break;const f=u-t;if(f<i&&(i=f,o=[...c],s=r.map(((t,e)=>n[e][t]&&n[e][t][2].length+n[e][t][3].length))),r[a]+=1,r[a]>=n[a].length&&(l[a]=!0,r[a]-=1,l.every((t=>t))))break}const a=o.map(((t,e)=>({pos:t,len:s[e]}))).filter((t=>t.pos>=0)).sort(((t,e)=>t.pos-e.pos)),u=a.length;return{A:t,M:a,D:u}}function k(){return w("span",{class:"morsels-ellipsis","aria-label":"ellipses"}," ... ")}function _(t,e,n){const{highlightRender:o}=n.uiOptions.resultsRenderOpts,{A:i,M:s}=t;if(!s.some((({pos:t})=>t>=0))){if(e){const t=i.trimStart().substring(0,80);return[80===t.length?t.replace(/\w+$/,""):t,k()]}return[i]}const r=[];let l=0;for(const{pos:t,len:c}of s){const s=t+c;if(t>l+80){e&&r.push(k());const l=i.substring(t-40,t);r.push(40===l.length?l.replace(/^\w+/,""):l),r.push(o(w,n,i.substring(t,s)))}else if(t>=l)r.pop(),r.push(i.substring(l,t)),r.push(o(w,n,i.substring(t,s)));else{if(!(s>l))continue;r.pop();r[r.length-1].textContent+=i.substring(l,s)}const a=i.substring(s,s+40);r.push(40===a.length?a.replace(/\w+$/,""):a),l=s}return e&&r.push(k()),r}function $(t,e,n,o){const{bodyOnlyRender:i,headingBodyRender:s}=o.uiOptions.resultsRenderOpts;let r,l=-2,c="",a=[];for(let n=0;n<t.length;n+=1){const[o,i]=t[n];switch(o){case"headingLink":l=n,c=i;break;case"heading":r=x(i,e),r.C=n,r.L=l===r.C-1?c:"",a.push({A:"",M:[],D:-2e3,N:r,L:r.L,C:n});break;case"body":{const t=x(i,e);r?(t.N=r,t.L=r.L,t.D+=r.D):t.D-=1e3,a.push(t);break}}}a.sort(((t,e)=>0===t.D&&0===e.D?e.A.length-t.A.length:e.D-t.D));const u=[],f=Math.min(a.length,2);for(let t=0;t<f&&a[t].D===a[0].D;t+=1)u.push(a[t]);return u.map((t=>{const e=_(t,!0,o);if(t.N){const i=_(t.N,!1,o),r=i.length?i:[t.N.A],l=t.L&&`${n}#${t.L}`;return s(w,o,r,e,l)}return i(w,o,e)}))}var O=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};const S=new DOMParser;function R(t,e,n,o,i,s){return O(this,void 0,void 0,(function*(){const{loaderConfigs:r}=n.indexingConfig,l=t.getFields();let c,a,u;for(const t of l){const[e,n]=t;switch(e){case"link":c=n;break;case"_relative_fp":a=n;break;case"title":u=n}if(c&&a&&u)break}const{sourceFilesUrl:f,resultsRenderOpts:{addSearchedTerms:d,listItemRender:h}}=e.uiOptions,m=c||"string"==typeof f&&a&&`${f}${a}`||"";u=u||a||c;let p,y=m;if(d&&m){const t=b(m);t.searchParams.append(d,i),y=t.toString()}if(o)p=$(l,s,y,e);else if(m)if(m.endsWith(".html")&&r.HtmlLoader){const t=yield(yield fetch(m)).text(),n=S.parseFromString(t,"text/html"),{title:o,bodies:i}=function(t,e,n,o,i){const s=[];if(e.exclude_selectors)for(const n of e.exclude_selectors){const e=t.querySelectorAll(n);for(let t=0;t<e.length;t+=1)e[t].remove()}e.selectors=e.selectors||[];const r=e.selectors.map((t=>t.selector)).join(",");!function t(n,o){for(const t of e.selectors)if(n.matches(t.selector)){Object.entries(t.attr_map).forEach((([t,e])=>{n.attributes[t]&&s.push([e,n.attributes[t].value])})),o=t.field_name;break}if(n.querySelector(r))for(let e=0;e<n.childNodes.length;e+=1){const i=n.childNodes[e];i.nodeType===Node.ELEMENT_NODE?t(i,o):i.nodeType===Node.TEXT_NODE&&o&&(s.length&&s[s.length-1][0]===o?s[s.length-1][1]+=i.data:s.push([o,i.data]))}else o&&(s.length&&s[s.length-1][0]===o?s[s.length-1][1]+=n.textContent:s.push([o,n.textContent||""]))}(t.documentElement,void 0);const l=s.find((t=>"title"===t[0]));let c="";return l&&([,c]=l),{title:c,bodies:$(s,n,o,i)}}(n,r.HtmlLoader,s,y,e);u=o||u,p=i}else if(m.endsWith(".txt")&&r.TxtLoader){p=$([["body",yield(yield fetch(m)).text()]],s,y,e)}else{const t=b(m);if(t.pathname.endsWith(".json")&&r.JsonLoader){const n=yield(yield fetch(m)).json(),{title:o,bodies:i}=function(t,e,n,o,i){const s=[],{field_map:r,field_order:l}=e,c=Object.entries(r).find((([,t])=>"title"===t)),a=c&&c[0];for(const e of l)e!==a&&t[e]&&s.push([r[e],t[e]]);return{title:a&&t[a],bodies:$(s,n,o,i)}}(t.hash?n[t.hash.substring(1)]:n,r.JsonLoader,s,y,e);u=o||u,p=i}}else p=[];return h(w,e,i,m,u,p,l)}))}function P(t,e,n,o,i){const s=[],r=[];for(const t of i.searchedTerms){const e=t.map((t=>(r.push(t),p(t)))).sort(((t,e)=>e.length-t.length)).join("|");if("ascii"===n.langConfig.lang){const t=new RegExp(`(^|\\W|_)(${e})((?=\\W|$))`,"gi");s.push(t)}else if("latin"===n.langConfig.lang){const t=new RegExp(`(^|\\W|_)(${e})(\\w*?)(?=\\W|$)`,"gi");s.push(t)}else if("chinese"===n.langConfig.lang){const t=new RegExp(`()(${e})()`,"gi");s.push(t)}}const l=n.fieldInfos.find((t=>t.do_store&&("body"===t.name||"title"===t.name||"heading"===t.name)));return Promise.all(o.map((t=>R(t,e,n,l,JSON.stringify(r),s))))}function j(t,e,n,o,i,s){return O(this,void 0,void 0,(function*(){if(t.W)return!1;const r=s.uiOptions.loadingIndicatorRender(w,s,!1,!0);o||i.appendChild(r),t.q&&t.q.disconnect();const l=document.createDocumentFragment();(o?s.uiOptions.termInfoRender(w,s,e.queryParts):[]).forEach((t=>l.appendChild(t)));const c=yield e.getNextN(s.uiOptions.resultsPerPage);if(t.W)return!1;const a=yield s.uiOptions.resultsRender(w,s,n,c,e);if(t.W)return!1;a.length?a.forEach((t=>l.appendChild(t))):o&&l.appendChild(s.uiOptions.noResultsRender(w,s));const u=l.lastElementChild;return o?(i.innerHTML="",t.U=g(),i.append(t.U),i.append(l)):r.replaceWith(l),a.length&&(t.q=new IntersectionObserver((([o],r)=>O(this,void 0,void 0,(function*(){o.isIntersecting&&(r.unobserve(u),yield j(t,e,n,!1,i,s))}))),{root:i,rootMargin:"150px 0px"}),t.q.observe(u)),!0}))}var T;!function(t){t.Auto="auto",t.Dropdown="dropdown",t.Fullscreen="fullscreen",t.Target="target"}(T||(T={}));class A{constructor(){this.B=!0,this.F=!1,this.U=g()}}function M(t,e,n){t.setAttribute("role","combobox"),t.setAttribute("aria-expanded","true"),t.setAttribute("aria-owns",e.getAttribute("id")),e.setAttribute("role","listbox"),e.setAttribute("aria-label",n),e.setAttribute("aria-live","polite")}function E(t,e){t.setAttribute("autocomplete","off"),t.setAttribute("aria-autocomplete","list"),t.setAttribute("aria-controls",e),t.setAttribute("aria-activedescendant","morsels-list-selected")}function D(t){return t.split("-")[0]}function C(t){return t.split("-")[1]}function L(t){return["top","bottom"].includes(D(t))?"x":"y"}function N(t){return"y"===t?"height":"width"}function W(t,e,n){let{reference:o,floating:i}=t;const s=o.x+o.width/2-i.width/2,r=o.y+o.height/2-i.height/2,l=L(e),c=N(l),a=o[c]/2-i[c]/2,u="x"===l;let f;switch(D(e)){case"top":f={x:s,y:o.y-i.height};break;case"bottom":f={x:s,y:o.y+o.height};break;case"right":f={x:o.x+o.width,y:r};break;case"left":f={x:o.x-i.width,y:r};break;default:f={x:o.x,y:o.y}}switch(C(e)){case"start":f[l]-=a*(n&&u?-1:1);break;case"end":f[l]+=a*(n&&u?-1:1)}return f}function q(t){return"number"!=typeof t?function(t){return{top:0,right:0,bottom:0,left:0,...t}}(t):{top:t,right:t,bottom:t,left:t}}function U(t){return{...t,top:t.y,left:t.x,right:t.x+t.width,bottom:t.y+t.height}}async function B(t,e){var n;void 0===e&&(e={});const{x:o,y:i,platform:s,rects:r,elements:l,strategy:c}=t,{boundary:a="clippingAncestors",rootBoundary:u="viewport",elementContext:f="floating",altBoundary:d=!1,padding:h=0}=e,m=q(h),p=l[d?"floating"===f?"reference":"floating":f],y=U(await s.getClippingRect({element:null==(n=await(null==s.isElement?void 0:s.isElement(p)))||n?p:p.contextElement||await(null==s.getDocumentElement?void 0:s.getDocumentElement(l.floating)),boundary:a,rootBoundary:u,strategy:c})),w=U(s.convertOffsetParentRelativeRectToViewportRelativeRect?await s.convertOffsetParentRelativeRectToViewportRelativeRect({rect:"floating"===f?{...r.floating,x:o,y:i}:r.reference,offsetParent:await(null==s.getOffsetParent?void 0:s.getOffsetParent(l.floating)),strategy:c}):r[f]);return{top:y.top-w.top+m.top,bottom:w.bottom-y.bottom+m.bottom,left:y.left-w.left+m.left,right:w.right-y.right+m.right}}const F=Math.min,I=Math.max;function H(t,e,n){return I(t,F(e,n))}const z=t=>({name:"arrow",options:t,async fn(e){const{element:n,padding:o=0}=null!=t?t:{},{x:i,y:s,placement:r,rects:l,platform:c}=e;if(null==n)return{};const a=q(o),u={x:i,y:s},f=L(r),d=N(f),h=await c.getDimensions(n),m="y"===f?"top":"left",p="y"===f?"bottom":"right",y=l.reference[d]+l.reference[f]-u[f]-l.floating[d],w=u[f]-l.reference[f],v=await(null==c.getOffsetParent?void 0:c.getOffsetParent(n));let g=v?"y"===f?v.clientHeight||0:v.clientWidth||0:0;0===g&&(g=l.floating[d]);const b=y/2-w/2,x=a[m],k=g-h[d]-a[p],_=g/2-h[d]/2+b,$=H(x,_,k);return{data:{[f]:$,centerOffset:_-$}}}}),J={left:"right",right:"left",bottom:"top",top:"bottom"};function Q(t){return t.replace(/left|right|bottom|top/g,(t=>J[t]))}function G(t,e,n){void 0===n&&(n=!1);const o=C(t),i=L(t),s=N(i);let r="x"===i?o===(n?"end":"start")?"right":"left":"start"===o?"bottom":"top";return e.reference[s]>e.floating[s]&&(r=Q(r)),{main:r,cross:Q(r)}}const V={start:"end",end:"start"};function X(t){return t.replace(/start|end/g,(t=>V[t]))}const Y=["top","right","bottom","left"],K=(Y.reduce(((t,e)=>t.concat(e,e+"-start",e+"-end")),[]),function(t){return void 0===t&&(t={}),{name:"flip",options:t,async fn(e){var n;const{placement:o,middlewareData:i,rects:s,initialPlacement:r,platform:l,elements:c}=e,{mainAxis:a=!0,crossAxis:u=!0,fallbackPlacements:f,fallbackStrategy:d="bestFit",flipAlignment:h=!0,...m}=t,p=D(o),y=f||(p!==r&&h?function(t){const e=Q(t);return[X(t),e,X(e)]}(r):[Q(r)]),w=[r,...y],v=await B(e,m),g=[];let b=(null==(n=i.flip)?void 0:n.overflows)||[];if(a&&g.push(v[p]),u){const{main:t,cross:e}=G(o,s,await(null==l.isRTL?void 0:l.isRTL(c.floating)));g.push(v[t],v[e])}if(b=[...b,{placement:o,overflows:g}],!g.every((t=>t<=0))){var x,k;const t=(null!=(x=null==(k=i.flip)?void 0:k.index)?x:0)+1,e=w[t];if(e)return{data:{index:t,overflows:b},reset:{placement:e}};let n="bottom";switch(d){case"bestFit":{var _;const t=null==(_=b.map((t=>[t,t.overflows.filter((t=>t>0)).reduce(((t,e)=>t+e),0)])).sort(((t,e)=>t[1]-e[1]))[0])?void 0:_[0].placement;t&&(n=t);break}case"initialPlacement":n=r}if(o!==n)return{reset:{placement:n}}}return{}}}});const Z=function(t){return void 0===t&&(t={}),{name:"size",options:t,async fn(e){const{placement:n,rects:o,platform:i,elements:s}=e,{apply:r,...l}=t,c=await B(e,l),a=D(n),u=C(n);let f,d;"top"===a||"bottom"===a?(f=a,d=u===(await(null==i.isRTL?void 0:i.isRTL(s.floating))?"start":"end")?"left":"right"):(d=a,f="end"===u?"top":"bottom");const h=I(c.left,0),m=I(c.right,0),p=I(c.top,0),y=I(c.bottom,0),w={availableHeight:o.floating.height-(["left","right"].includes(n)?2*(0!==p||0!==y?p+y:I(c.top,c.bottom)):c[f]),availableWidth:o.floating.width-(["top","bottom"].includes(n)?2*(0!==h||0!==m?h+m:I(c.left,c.right)):c[d])},v=await i.getDimensions(s.floating);null==r||r({...e,...w});const g=await i.getDimensions(s.floating);return v.width!==g.width||v.height!==g.height?{reset:{rects:!0}}:{}}}};function tt(t){return t&&t.document&&t.location&&t.alert&&t.setInterval}function et(t){if(null==t)return window;if(!tt(t)){const e=t.ownerDocument;return e&&e.defaultView||window}return t}function nt(t){return et(t).getComputedStyle(t)}function ot(t){return tt(t)?"":t?(t.nodeName||"").toLowerCase():""}function it(){const t=navigator.userAgentData;return null!=t&&t.brands?t.brands.map((t=>t.brand+"/"+t.version)).join(" "):navigator.userAgent}function st(t){return t instanceof et(t).HTMLElement}function rt(t){return t instanceof et(t).Element}function lt(t){return"undefined"!=typeof ShadowRoot&&(t instanceof et(t).ShadowRoot||t instanceof ShadowRoot)}function ct(t){const{overflow:e,overflowX:n,overflowY:o}=nt(t);return/auto|scroll|overlay|hidden/.test(e+o+n)}function at(t){return["table","td","th"].includes(ot(t))}function ut(t){const e=/firefox/i.test(it()),n=nt(t);return"none"!==n.transform||"none"!==n.perspective||"paint"===n.contain||["transform","perspective"].includes(n.willChange)||e&&"filter"===n.willChange||e&&!!n.filter&&"none"!==n.filter}function ft(){return!/^((?!chrome|android).)*safari/i.test(it())}const dt=Math.min,ht=Math.max,mt=Math.round;function pt(t,e,n){var o,i,s,r;void 0===e&&(e=!1),void 0===n&&(n=!1);const l=t.getBoundingClientRect();let c=1,a=1;e&&st(t)&&(c=t.offsetWidth>0&&mt(l.width)/t.offsetWidth||1,a=t.offsetHeight>0&&mt(l.height)/t.offsetHeight||1);const u=rt(t)?et(t):window,f=!ft()&&n,d=(l.left+(f&&null!=(o=null==(i=u.visualViewport)?void 0:i.offsetLeft)?o:0))/c,h=(l.top+(f&&null!=(s=null==(r=u.visualViewport)?void 0:r.offsetTop)?s:0))/a,m=l.width/c,p=l.height/a;return{width:m,height:p,top:h,right:d+m,bottom:h+p,left:d,x:d,y:h}}function yt(t){return(e=t,(e instanceof et(e).Node?t.ownerDocument:t.document)||window.document).documentElement;var e}function wt(t){return rt(t)?{scrollLeft:t.scrollLeft,scrollTop:t.scrollTop}:{scrollLeft:t.pageXOffset,scrollTop:t.pageYOffset}}function vt(t){return pt(yt(t)).left+wt(t).scrollLeft}function gt(t,e,n){const o=st(e),i=yt(e),s=pt(t,o&&function(t){const e=pt(t);return mt(e.width)!==t.offsetWidth||mt(e.height)!==t.offsetHeight}(e),"fixed"===n);let r={scrollLeft:0,scrollTop:0};const l={x:0,y:0};if(o||!o&&"fixed"!==n)if(("body"!==ot(e)||ct(i))&&(r=wt(e)),st(e)){const t=pt(e,!0);l.x=t.x+e.clientLeft,l.y=t.y+e.clientTop}else i&&(l.x=vt(i));return{x:s.left+r.scrollLeft-l.x,y:s.top+r.scrollTop-l.y,width:s.width,height:s.height}}function bt(t){return"html"===ot(t)?t:t.assignedSlot||t.parentNode||(lt(t)?t.host:null)||yt(t)}function xt(t){return st(t)&&"fixed"!==getComputedStyle(t).position?t.offsetParent:null}function kt(t){const e=et(t);let n=xt(t);for(;n&&at(n)&&"static"===getComputedStyle(n).position;)n=xt(n);return n&&("html"===ot(n)||"body"===ot(n)&&"static"===getComputedStyle(n).position&&!ut(n))?e:n||function(t){let e=bt(t);for(lt(e)&&(e=e.host);st(e)&&!["html","body"].includes(ot(e));){if(ut(e))return e;e=e.parentNode}return null}(t)||e}function _t(t){if(st(t))return{width:t.offsetWidth,height:t.offsetHeight};const e=pt(t);return{width:e.width,height:e.height}}function $t(t){const e=bt(t);return["html","body","#document"].includes(ot(e))?t.ownerDocument.body:st(e)&&ct(e)?e:$t(e)}function Ot(t,e){var n;void 0===e&&(e=[]);const o=$t(t),i=o===(null==(n=t.ownerDocument)?void 0:n.body),s=et(o),r=i?[s].concat(s.visualViewport||[],ct(o)?o:[]):o,l=e.concat(r);return i?l:l.concat(Ot(r))}function St(t,e,n){return"viewport"===e?U(function(t,e){const n=et(t),o=yt(t),i=n.visualViewport;let s=o.clientWidth,r=o.clientHeight,l=0,c=0;if(i){s=i.width,r=i.height;const t=ft();(t||!t&&"fixed"===e)&&(l=i.offsetLeft,c=i.offsetTop)}return{width:s,height:r,x:l,y:c}}(t,n)):rt(e)?function(t,e){const n=pt(t,!1,"fixed"===e),o=n.top+t.clientTop,i=n.left+t.clientLeft;return{top:o,left:i,x:i,y:o,right:i+t.clientWidth,bottom:o+t.clientHeight,width:t.clientWidth,height:t.clientHeight}}(e,n):U(function(t){var e;const n=yt(t),o=wt(t),i=null==(e=t.ownerDocument)?void 0:e.body,s=ht(n.scrollWidth,n.clientWidth,i?i.scrollWidth:0,i?i.clientWidth:0),r=ht(n.scrollHeight,n.clientHeight,i?i.scrollHeight:0,i?i.clientHeight:0);let l=-o.scrollLeft+vt(t);const c=-o.scrollTop;return"rtl"===nt(i||n).direction&&(l+=ht(n.clientWidth,i?i.clientWidth:0)-s),{width:s,height:r,x:l,y:c}}(yt(t)))}function Rt(t){const e=Ot(t),n=["absolute","fixed"].includes(nt(t).position)&&st(t)?kt(t):t;return rt(n)?e.filter((t=>rt(t)&&function(t,e){const n=null==e||null==e.getRootNode?void 0:e.getRootNode();if(null!=t&&t.contains(e))return!0;if(n&&lt(n)){let n=e;do{if(n&&t===n)return!0;n=n.parentNode||n.host}while(n)}return!1}(t,n)&&"body"!==ot(t))):[]}const Pt={getClippingRect:function(t){let{element:e,boundary:n,rootBoundary:o,strategy:i}=t;const s=[..."clippingAncestors"===n?Rt(e):[].concat(n),o],r=s[0],l=s.reduce(((t,n)=>{const o=St(e,n,i);return t.top=ht(o.top,t.top),t.right=dt(o.right,t.right),t.bottom=dt(o.bottom,t.bottom),t.left=ht(o.left,t.left),t}),St(e,r,i));return{width:l.right-l.left,height:l.bottom-l.top,x:l.left,y:l.top}},convertOffsetParentRelativeRectToViewportRelativeRect:function(t){let{rect:e,offsetParent:n,strategy:o}=t;const i=st(n),s=yt(n);if(n===s)return e;let r={scrollLeft:0,scrollTop:0};const l={x:0,y:0};if((i||!i&&"fixed"!==o)&&(("body"!==ot(n)||ct(s))&&(r=wt(n)),st(n))){const t=pt(n,!0);l.x=t.x+n.clientLeft,l.y=t.y+n.clientTop}return{...e,x:e.x-r.scrollLeft+l.x,y:e.y-r.scrollTop+l.y}},isElement:rt,getDimensions:_t,getOffsetParent:kt,getDocumentElement:yt,getElementRects:t=>{let{reference:e,floating:n,strategy:o}=t;return{reference:gt(e,kt(n),o),floating:{..._t(n),x:0,y:0}}},getClientRects:t=>Array.from(t.getClientRects()),isRTL:t=>"rtl"===nt(t).direction};const jt=(t,e,n)=>(async(t,e,n)=>{const{placement:o="bottom",strategy:i="absolute",middleware:s=[],platform:r}=n,l=await(null==r.isRTL?void 0:r.isRTL(e));let c=await r.getElementRects({reference:t,floating:e,strategy:i}),{x:a,y:u}=W(c,o,l),f=o,d={};for(let n=0;n<s.length;n++){const{name:h,fn:m}=s[n],{x:p,y,data:w,reset:v}=await m({x:a,y:u,initialPlacement:o,placement:f,strategy:i,middlewareData:d,rects:c,platform:r,elements:{reference:t,floating:e}});a=null!=p?p:a,u=null!=y?y:u,d={...d,[h]:{...d[h],...w}},v&&("object"==typeof v&&(v.placement&&(f=v.placement),v.rects&&(c=!0===v.rects?await r.getElementRects({reference:t,floating:e,strategy:i}):v.rects),({x:a,y:u}=W(c,f,l))),n=-1)}return{x:a,y:u,placement:f,strategy:i,middlewareData:d}})(t,e,{platform:Pt,...n});function Tt(t,e,n){if(!1===e.tip)return;function o(t,e){return w("li",{class:"morsels-tip-item"},t,w("code",{},e))}const i=w("ul",{class:"morsels-tip-list"},o('Match multiple terms with "AND": ',"weather AND forecast AND sunny"),o('Flip results with "NOT": ',"NOT rainy"),o("Match 1 of 3 specific parts of pages: ","title:forecast or heading:sunny or body:rainy"),o("Group terms or expressions into a expression with brackets: ","(...expressions...)")),s=w("div",{class:"morsels-tip-popup-root"},w("div",{class:"morsels-tip-popup"},w("div",{class:"morsels-tip-popup-title"},"🔎 Didn't find what you needed?"),i),w("div",{class:"morsels-tip-popup-separator"}));function r(){Object.assign(s.style,{left:"calc(var(--morsels-tip-icon-size) - 150px)",top:"-160px"}),s.classList.remove("shown")}r();const l=w("div",{class:"morsels-tip-root",tabindex:"0"},w("span",{class:"morsels-tip-icon"},"?"),s);function c(){jt(l,s,{placement:"top-end",middleware:[K({crossAxis:!1,flipAlignment:!1,padding:10})]}).then((({x:t,y:e})=>{Object.assign(s.style,{left:`${t}px`,top:`${e}px`}),s.classList.add("shown")}))}l.onmouseover=c,l.onfocus=c,l.onmouseleave=r,l.onblur=r,t.append(l),n.setupPromise.then((()=>{n.cfg.indexingConfig.withPositions&&i.append(o("Search for phrases using quotes: ",'"for tomorrow"'))}))}function At(t,e){t.setAttribute("autocomplete","off"),t.setAttribute("readonly",""),t.setAttribute("role","button"),t.setAttribute("aria-label",e),t.classList.add("morsels-button-input")}var Mt=function(t,e,n,o){return new(n||(n=Promise))((function(i,s){function r(t){try{c(o.next(t))}catch(t){s(t)}}function l(t){try{c(o.throw(t))}catch(t){s(t)}}function c(t){var e;t.done?i(t.value):(e=t.value,e instanceof n?e:new n((function(t){t(e)}))).then(r,l)}c((o=o.apply(t,e||[])).next())}))};let Et,Dt,Ct=!1,Lt=!1,Nt=!1;function Wt(t){return t.mode===T.Auto&&!Ct||t.mode===T.Dropdown}function qt(t,e,n,o){const{uiOptions:i}=o,s=new A;function r(r){var l;return Mt(this,void 0,void 0,(function*(){s.F=!0;const c=i.loadingIndicatorRender(w,o,!1,s.B);s.U.replaceWith(c),s.U=c;try{null===(l=s.currQuery)||void 0===l||l.free(),s.currQuery=yield n.getQuery(r);(yield j(s,s.currQuery,n.cfg,!0,e,o))&&(s.B=!1),t.scrollTo({top:0}),e.scrollTo({top:0})}catch(t){throw console.error(t),e.innerHTML="",e.appendChild(i.errorRender(w,o)),t}finally{if(s.W){const t=s.W;s.W=void 0,yield t()}else s.F=!1}}))}n.setupPromise.then((()=>{s.W&&(s.W(),s.W=void 0)}));let l=-1;return t=>{const c=i.preprocessQuery(t.target.value);if(clearTimeout(l),c.length)l=setTimeout((()=>{var t;s.B&&!(null===(t=e.firstElementChild)||void 0===t?void 0:t.getAttribute(v))&&(e.innerHTML="",s.U=i.loadingIndicatorRender(w,o,!n.isSetupDone,!0),e.appendChild(s.U),Wt(i)&&Et()),s.F||!n.isSetupDone?s.W=()=>r(c):r(c)}),i.inputDebounce);else{const t=()=>{e.innerHTML="",i.mode!==T.Target&&(Wt(i)?Dt():e.appendChild(i.fsBlankRender(w,o))),s.F=!1,s.B=!0};s.F?s.W=t:t()}}}const Ut=function(t){const e=t.isMobileDevice||(()=>window.matchMedia("only screen and (max-width: 1024px)").matches);Ct=e(),function(t,e){t.searcherOptions=t.searcherOptions||{};const{searcherOptions:n}=t;if(!("url"in n))throw new Error("Mandatory url parameter not specified");n.url.endsWith("/")||(n.url+="/"),n.url.startsWith("/")&&(n.url=window.location.origin+n.url),"numberOfExpandedTerms"in n||(n.numberOfExpandedTerms=3),"useQueryTermProximity"in n||(n.useQueryTermProximity=!e),"resultLimit"in n||(n.resultLimit=null),t.uiOptions=t.uiOptions||{};const{uiOptions:o}=t;if(o.sourceFilesUrl&&!o.sourceFilesUrl.endsWith("/")&&(o.sourceFilesUrl+="/"),o.mode=o.mode||T.Auto,o.mode===T.Target&&("string"==typeof o.target&&(o.target=document.getElementById(o.target)),!o.target))throw new Error("'target' mode specified but no valid target option specified");if("input"in o&&"string"!=typeof o.input||(o.input=document.getElementById(o.input||"morsels-search")),[T.Dropdown,T.Target].includes(o.mode)&&!o.input)throw new Error("'dropdown' or 'target' mode specified but no input element found");"inputDebounce"in o||(o.inputDebounce=100),o.preprocessQuery=o.preprocessQuery||(t=>t),o.dropdownAlignment=o.dropdownAlignment||"bottom-end","string"==typeof o.fsContainer&&(o.fsContainer=document.getElementById(o.fsContainer)),o.fsContainer=o.fsContainer||document.getElementsByTagName("body")[0],o.resultsPerPage=o.resultsPerPage||8,o.label=o.label||"Search this site",o.resultsLabel=o.resultsLabel||"Site results",o.fsInputLabel=o.fsInputLabel||"Search",o.fsPlaceholder=o.fsPlaceholder||"Search this site...",o.fsCloseText=o.fsCloseText||"Close",o.errorRender=o.errorRender||(t=>t("div",{class:"morsels-error"},"Oops! Something went wrong... 🙁")),o.noResultsRender=o.noResultsRender||(t=>t("div",{class:"morsels-no-results"},"No results found")),o.fsBlankRender=o.fsBlankRender||(t=>t("div",{class:"morsels-fs-blank"},"Start Searching Above!")),o.loadingIndicatorRender||(o.loadingIndicatorRender=(t,e,n,o)=>{const i=t("span",{class:"morsels-loading-indicator"});if(n){const e=t("div",{class:"morsels-initialising-text"},"... Initialising ...");return t("div",{class:"morsels-initialising"},e,i)}return o||i.classList.add("morsels-loading-indicator-subsequent"),i});const i=o.loadingIndicatorRender;o.loadingIndicatorRender=(...t)=>{const e=i(...t);return e.setAttribute(v,"true"),e},o.termInfoRender=o.termInfoRender||(()=>[]),o.resultsRender=o.resultsRender||P,o.resultsRenderOpts=o.resultsRenderOpts||{};const{resultsRenderOpts:s}=o;s.listItemRender=s.listItemRender||((t,e,n,o,i,r)=>{const l=t("a",{class:"morsels-link"},t("div",{class:"morsels-title"},i),...r);if(o){let t=o;const{addSearchedTerms:e}=s;if(e){const i=b(o);i.searchParams.append(e,n),t=i.toString()}l.setAttribute("href",t)}return t("li",{class:"morsels-list-item",role:"option"},l)}),s.headingBodyRender=s.headingBodyRender||((t,e,n,o,i)=>{const s=t("a",{class:"morsels-heading-body"},t("div",{class:"morsels-heading"},...n),t("div",{class:"morsels-bodies"},t("div",{class:"morsels-body"},...o)));return i&&s.setAttribute("href",i),s}),s.bodyOnlyRender=s.bodyOnlyRender||((t,e,n)=>t("div",{class:"morsels-body"},...n)),s.highlightRender=s.highlightRender||((t,e,n)=>t("span",{class:"morsels-highlight"},n)),t.otherOptions=t.otherOptions||{}}(t,Ct);const{uiOptions:n}=t,{input:o,mode:i,dropdownAlignment:s,label:r,fsInputLabel:l,target:c}=n,a=new m(t.searcherOptions),[u,f,d,h,p]=function(t,e,n){const{uiOptions:o}=t,i=w("input",{class:"morsels-fs-input",type:"search",placeholder:o.fsPlaceholder,"aria-labelledby":"morsels-fs-label",enterkeyhint:"search"});E(i,"morsels-fs-list");const s=w("button",{class:"morsels-input-close-fs"},o.fsCloseText),r=w("ul",{id:"morsels-fs-list",class:"morsels-list","aria-labelledby":"morsels-fs-label"}),l=w("div",{class:"morsels-root morsels-fs-root"},w("form",{class:"morsels-fs-input-button-wrapper"},w("label",{id:"morsels-fs-label",for:"morsels-fs-input",style:"display: none"},o.label),i,s),r);l.onclick=t=>t.stopPropagation(),l.onmousedown=t=>t.stopPropagation(),M(l,r,o.label),Tt(l,o,e);const c=w("div",{class:"morsels-fs-backdrop"},l);function a(t){n(t),c.remove()}return c.onmousedown=()=>a(!1),c.onkeyup=t=>{"Escape"===t.code&&(t.stopPropagation(),a(!0))},s.onclick=t=>{t.preventDefault(),a(""===t.pointerType)},[c,r,i,function(){o.fsContainer.appendChild(c);const t=c.querySelector("input.morsels-fs-input");t&&t.focus();const e=r.querySelector(".focus");e&&r.scrollTo({top:e.offsetTop-r.offsetTop-30})},a]}(t,a,(t=>{t&&o&&o.focus(),Nt=!1}));function y(){Nt||(h(),Nt=!0)}function g(){p(!1)}function x(){function t(){Wt(n)||y()}o.addEventListener("click",t),o.addEventListener("keydown",(e=>{"Enter"===e.key&&t()}))}let k;if(d.addEventListener("input",qt(u,f,a,t)),f.appendChild(n.fsBlankRender(w,t)),!o||i!==T.Auto&&i!==T.Dropdown){if(o&&i===T.Fullscreen)At(o,l),x();else if(o&&i===T.Target){o.addEventListener("input",qt(c,c,a,t));let e=c.getAttribute("id");e||(c.setAttribute("id","morsels-target-list"),e="morsels-target-list"),E(o,e),M(o,c,n.label)}}else{const c=o.parentElement;o.remove();const[u,f]=function(t,e,n){const o=w("ul",{id:"morsels-dropdown-list",class:"morsels-list"}),i=w("div",{class:"morsels-inner-root",style:"display: none;"},w("div",{class:"morsels-input-dropdown-separator"}),o);return Tt(i,t,e),[w("div",{class:"morsels-root"},n,i),o]}(n,a,o);function d(){i!==T.Dropdown&&(Ct=e())?(Dt(),function(t,e,n,o){t.removeAttribute("role"),t.removeAttribute("aria-expanded"),t.removeAttribute("aria-owns"),e.removeAttribute("role"),e.removeAttribute("aria-label"),e.removeAttribute("aria-live"),n.removeAttribute("aria-autocomplete"),n.removeAttribute("aria-controls"),n.removeAttribute("aria-activedescendant"),At(n,o)}(u,k,o,l)):(g(),Dt(),document.activeElement===o&&Et(),function(t,e,n,o){var i;(i=t).removeAttribute("readonly"),i.removeAttribute("role"),i.removeAttribute("aria-label"),i.classList.remove("morsels-button-input"),E(t,"morsels-dropdown-list"),M(e,n,o)}(o,u,k,r))}let h;k=f,c.appendChild(u),Et=()=>{!function(t,e,n){if(e.childElementCount){const o=t.children[1],i=o.firstElementChild;o.style.display="block",jt(t,o,{placement:n,middleware:[K({padding:10,mainAxis:!1}),Z({apply({availableWidth:t,availableHeight:n}){Object.assign(e.style,{maxWidth:`min(${t}px, var(--morsels-dropdown-max-width))`,maxHeight:`min(${n}px, var(--morsels-dropdown-max-height))`})},padding:10}),z({element:i})]}).then((({x:t,y:e,middlewareData:n})=>{Object.assign(o.style,{left:`${t}px`,top:`${e}px`});const{x:s}=n.arrow;Object.assign(i.style,{left:null!=s?`${s}px`:""})}))}}(u,k,s),Lt=!0},Dt=()=>{u.children[1].style.display="none",Lt=!1},o.addEventListener("input",qt(u,k,a,t)),d(),window.addEventListener("resize",(()=>{clearTimeout(h),h=setTimeout(d,10)})),o.addEventListener("blur",(()=>{Wt(n)&&setTimeout((()=>{let t=document.activeElement;for(;t;)if(t=t.parentElement,t===u)return void o.focus();Dt()}),100)})),o.addEventListener("focus",(()=>Wt(n)&&Et())),x()}function _(t){if(!["ArrowDown","ArrowUp","Home","End","Enter"].includes(t.key))return;let e,o=t=>{const n=t.offsetTop-e.offsetTop-e.clientHeight/2+t.clientHeight/2;e.scrollTo({top:n})};if(Wt(n)){if(!Lt)return;e=k}else if(i===T.Target)e=c,o=t=>{t.scrollIntoView({block:"center"})};else{if(!Nt)return;e=f}const s=e.querySelector(".focus");function r(t){s&&(s.classList.remove("focus"),s.removeAttribute("aria-selected"),s.removeAttribute("id")),t.classList.add("focus"),t.setAttribute("aria-selected","true"),t.setAttribute("id","morsels-list-selected"),o(t)}function l(t,e){t&&!t.getAttribute(v)?r(t):e&&!e.getAttribute(v)&&r(e)}const a=e.firstElementChild,u=e.lastElementChild;if("ArrowDown"===t.key)s?l(s.nextElementSibling,null):l(a,null==a?void 0:a.nextElementSibling);else if("ArrowUp"===t.key)s&&l(s.previousElementSibling,null);else if("Home"===t.key)l(a,null==a?void 0:a.nextElementSibling);else if("End"===t.key)l(u,null==u?void 0:u.previousElementSibling);else if("Enter"===t.key&&s){const t=s.querySelector("a[href]");t&&(window.location.href=t.getAttribute("href"))}t.preventDefault()}return null==o||o.addEventListener("keydown",_),d.addEventListener("keydown",_),{showFullscreen:y,hideFullscreen:g}};return e=e.default})()}));