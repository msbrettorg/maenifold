import{g as j}from"./index-BSZSC1U4.js";function N(u,g){for(var m=0;m<g.length;m++){const c=g[m];if(typeof c!="string"&&!Array.isArray(c)){for(const a in c)if(a!=="default"&&!(a in u)){const p=Object.getOwnPropertyDescriptor(c,a);p&&Object.defineProperty(u,a,p.get?p:{enumerable:!0,get:()=>c[a]})}}}return Object.freeze(Object.defineProperty(u,Symbol.toStringTag,{value:"Module"}))}var S={exports:{}},_;function A(){return _||(_=1,(function(u,g){(function(c,a){u.exports=a()})(self,()=>(()=>{var m={466:(o=>{o.exports=`/******/ (() => { // webpackBootstrap
/******/ 	"use strict";

// UNUSED EXPORTS: default

;// ./lib/object-path.ts
const PATH_REG = /([.[\\]:;'"\\s])/;
function escapePathPart(pathPart) {
    if (!PATH_REG.test(pathPart)) {
        return pathPart;
    }
    const escaped = pathPart.replace(new RegExp(PATH_REG.source, 'g'), '\\\\$1');
    return \`["\${escaped}"]\`;
}
function unescapePathPart(pathPart) {
    return pathPart.replace(/^\\["/, '').replace(/"]$/, '').replace(/\\\\/, '');
}
function splitPath(path) {
    const result = [];
    let lastEnd = 0;
    for (let i = 0; i < path.length; i++) {
        const char = path[i];
        if (PATH_REG.test(char) && path[i - 1] !== '\\\\') {
            result.push(path.substring(lastEnd, i));
            lastEnd = i + 1;
        }
    }
    result.push(path.substring(lastEnd, path.length));
    return result.filter(pathPart => !!pathPart).map(pathPart => pathPart.replace(/\\\\/g, ''));
}
/**
 * Extracts object property value by given path. Supports nested and array values: 'foo[0].bar'
 * @param {Object} object source object
 * @param {string} path path to value
 * @return {any | null} value by given path
 * */
function propertyByPath(object, path) {
    return splitPath(path).reduce((acc, pathPart) => {
        if (acc) {
            return acc[pathPart];
        }
        return null;
    }, object);
}

;// ./lib/connection.ts

const TYPE_MESSAGE = 'message';
const TYPE_RESPONSE = 'response';
const TYPE_SET_INTERFACE = 'set-interface';
const TYPE_SERVICE_MESSAGE = 'service-message';
// @ts-expect-error this is IE11 obsolete check. It is not typed
const isIE11 = !!window.MSInputMethodContext && !!document.documentMode;
const defaultOptions = {
    //Will not affect IE11 because there sandboxed iframe has not 'null' origin
    //but base URL of iframe's src
    allowedSenderOrigin: undefined
};
class Connection {
    constructor(postMessage, registerOnMessageListener, options = {}) {
        this.remote = {};
        this.serviceMethods = {};
        this.localApi = {};
        this.callbacks = {};
        this._resolveRemoteMethodsPromise = null;
        this.options = Object.assign(Object.assign({}, defaultOptions), options);
        //Random number between 0 and 100000
        this.incrementalID = Math.floor(Math.random() * 100000);
        this.postMessage = postMessage;
        this.remoteMethodsWaitPromise = new Promise(resolve => {
            this._resolveRemoteMethodsPromise = resolve;
        });
        registerOnMessageListener((e) => this.onMessageListener(e));
    }
    /**
       * Listens to remote messages. Calls local method if it is called outside or call stored callback if it is response.
       * @param e - onMessage event
       */
    onMessageListener(e) {
        const data = e.data;
        const { allowedSenderOrigin } = this.options;
        if (allowedSenderOrigin && e.origin !== allowedSenderOrigin && !isIE11) {
            return;
        }
        if (data.type === TYPE_RESPONSE) {
            this.popCallback(data.callId, data.success, data.result);
        }
        else if (data.type === TYPE_MESSAGE) {
            this
                .callLocalApi(data.methodName, data.arguments)
                .then(res => this.responseOtherSide(data.callId, res))
                .catch(err => this.responseOtherSide(data.callId, err, false));
        }
        else if (data.type === TYPE_SET_INTERFACE) {
            this.setInterface(data.apiMethods);
            this.responseOtherSide(data.callId);
        }
        else if (data.type === TYPE_SERVICE_MESSAGE) {
            this
                .callLocalServiceMethod(data.methodName, data.arguments)
                .then(res => this.responseOtherSide(data.callId, res))
                .catch(err => this.responseOtherSide(data.callId, err, false));
        }
    }
    postMessageToOtherSide(dataToPost) {
        this.postMessage(dataToPost, '*');
    }
    /**
       * Sets remote interface methods
       * @param remote - hash with keys of remote API methods. Values is ignored
       */
    setInterface(remoteMethods) {
        var _a;
        this.remote = {};
        remoteMethods.forEach((key) => {
            // If key is nested, we need to create nested structure
            const parts = splitPath(key);
            let current = this.remote;
            for (let i = 0; i < parts.length - 1; i++) {
                const part = parts[i];
                if (!current[part] || typeof current[part] !== 'object') {
                    current[part] = {};
                }
                current = current[part];
            }
            current[parts[parts.length - 1]] = this.createMethodWrapper(key);
        });
        (_a = this._resolveRemoteMethodsPromise) === null || _a === void 0 ? void 0 : _a.call(this);
    }
    getMethodsFromInterface(api) {
        return Object.keys(api).reduce((acc, key) => {
            if (typeof api[key] === 'object') {
                acc.push(...this.getMethodsFromInterface(api[key]).map(subKey => \`\${key}.\${subKey}\`));
            }
            else {
                acc.push(key);
            }
            return acc;
        }, []);
    }
    setLocalApi(api) {
        return new Promise((resolve, reject) => {
            const id = this.registerCallback(resolve, reject);
            this.postMessageToOtherSide({
                callId: id,
                apiMethods: this.getMethodsFromInterface(api),
                type: TYPE_SET_INTERFACE
            });
        }).then(() => this.localApi = api);
    }
    setServiceMethods(api) {
        this.serviceMethods = api;
    }
    /**
       * Calls local method
       * @param methodName
       * @param args
       * @returns {Promise.<*>|string}
       */
    callLocalApi(methodName, args) {
        const method = propertyByPath(this.localApi, methodName);
        if (!method) {
            throw new Error(\`Local method "\${methodName}" is not registered\`);
        }
        return Promise.resolve(method.call(this, ...args));
    }
    /**
       * Calls local method registered as "service method"
       * @param methodName
       * @param args
       * @returns {Promise.<*>}
       */
    callLocalServiceMethod(methodName, args) {
        const method = propertyByPath(this.serviceMethods, methodName);
        if (!method) {
            throw new Error(\`Service method \${methodName} is not registered\`);
        }
        return Promise.resolve(method.call(this, ...args));
    }
    /**
       * Wraps remote method with callback storing code
       * @param methodName - method to wrap
       * @returns {Function} - function to call as remote API interface
       */
    createMethodWrapper(methodName) {
        return (...args) => {
            return this.callRemoteMethod(methodName, ...args);
        };
    }
    /**
       * Calls other side with arguments provided
       * @param id
       * @param methodName
       * @param args
       */
    callRemoteMethod(methodName, ...args) {
        return new Promise((resolve, reject) => {
            const id = this.registerCallback(resolve, reject);
            this.postMessageToOtherSide({
                callId: id,
                methodName: methodName,
                type: TYPE_MESSAGE,
                arguments: args
            });
        });
    }
    /**
       * Calls remote service method
       * @param methodName
       * @param args
       * @returns {*}
       */
    callRemoteServiceMethod(methodName, ...args) {
        return new Promise((resolve, reject) => {
            const id = this.registerCallback(resolve, reject);
            this.postMessageToOtherSide({
                callId: id,
                methodName: methodName,
                type: TYPE_SERVICE_MESSAGE,
                arguments: args
            });
        });
    }
    /**
       * Respond to remote call
       * @param id - remote call ID
       * @param result - result to pass to calling function
       */
    responseOtherSide(id, result, success = true) {
        if (result instanceof Error) {
            // Error could be non-serializable, so we copy properties manually
            result = [...Object.keys(result), 'message'].reduce((acc, it) => {
                acc[it] = result[it];
                return acc;
            }, {});
        }
        const doPost = () => this.postMessage({
            callId: id,
            type: TYPE_RESPONSE,
            success,
            result
        }, '*');
        try {
            doPost();
        }
        catch (err) {
            console.error('Failed to post response, recovering...', err); // eslint-disable-line no-console
            if (err instanceof DOMException) {
                result = JSON.parse(JSON.stringify(result));
                doPost();
            }
        }
    }
    /*
       * Stores callbacks to call later when remote call will be answered
       */
    registerCallback(successCallback, failureCallback) {
        const id = (++this.incrementalID).toString();
        this.callbacks[id] = { successCallback, failureCallback };
        return id;
    }
    /**
       * Calls and delete stored callback
       * @param id - call id
       * @param success - was call successful
       * @param result - result of remote call
       */
    popCallback(id, success, result) {
        const callback = this.callbacks[id];
        if (!callback) {
            return;
        }
        if (success) {
            callback.successCallback(result);
        }
        else {
            callback.failureCallback(result);
        }
        delete this.callbacks[id];
    }
}
/* harmony default export */ const connection = (Connection);

;// ./node_modules/ts-loader/index.js??ruleSet[1].rules[0]!./lib/frame.ts

class Frame {
    constructor() {
        this.connection = new connection(window.parent.postMessage.bind(window.parent), listener => {
            const sourceCheckListener = (event) => {
                if (event.source !== window.parent) {
                    return;
                }
                return listener(event);
            };
            window.addEventListener('message', sourceCheckListener);
        });
        this.connection.setServiceMethods({
            runCode: (code) => this.runCode(code),
            importScript: (path) => this.importScript(path),
            injectStyle: (style) => this.injectStyle(style),
            importStyle: (path) => this.importStyle(path)
        });
        this.connection.callRemoteServiceMethod('iframeInitialized');
    }
    /**
       * Creates script tag with passed code and attaches it. Runs synchronous
       * @param code
       */
    runCode(code) {
        const scriptTag = document.createElement('script');
        scriptTag.innerHTML = code;
        document.getElementsByTagName('head')[0].appendChild(scriptTag);
    }
    importScript(scriptUrl) {
        const scriptTag = document.createElement('script');
        scriptTag.src = scriptUrl;
        document.getElementsByTagName('head')[0].appendChild(scriptTag);
        return new Promise(resolve => scriptTag.onload = () => resolve());
    }
    injectStyle(style) {
        const styleTag = document.createElement('style');
        styleTag.innerHTML = style;
        document.getElementsByTagName('head')[0].appendChild(styleTag);
    }
    importStyle(styleUrl) {
        const linkTag = document.createElement('link');
        linkTag.rel = 'stylesheet';
        linkTag.href = styleUrl;
        document.getElementsByTagName('head')[0].appendChild(linkTag);
    }
}
// @ts-expect-error we explicitly export library to global namespace because
const Websandbox = window.Websandbox || new Frame();
// @ts-expect-error we explicitly export library to global namespace because
// Webpack won't do it for us when this file is loaded via code-loader
window.Websandbox = Websandbox;
/* harmony default export */ const ts_loader_ruleSet_1_rules_0_lib_frame = ((/* unused pure expression or super */ null && (Websandbox)));

/******/ })()
;
//# sourceMappingURL=compile-loader-file-name.js.map`})},c={};function a(o){var i=c[o];if(i!==void 0)return i.exports;var l=c[o]={exports:{}};return m[o](l,l.exports,a),l.exports}a.n=o=>{var i=o&&o.__esModule?()=>o.default:()=>o;return a.d(i,{a:i}),i},a.d=(o,i)=>{for(var l in i)a.o(i,l)&&!a.o(o,l)&&Object.defineProperty(o,l,{enumerable:!0,get:i[l]})},a.o=(o,i)=>Object.prototype.hasOwnProperty.call(o,i),a.r=o=>{typeof Symbol<"u"&&Symbol.toStringTag&&Object.defineProperty(o,Symbol.toStringTag,{value:"Module"}),Object.defineProperty(o,"__esModule",{value:!0})};var p={};return(()=>{a.r(p),a.d(p,{BaseOptions:()=>w,default:()=>R});const o=/([.[\]:;'"\s])/;function i(d){const e=[];let t=0;for(let n=0;n<d.length;n++){const r=d[n];o.test(r)&&d[n-1]!=="\\"&&(e.push(d.substring(t,n)),t=n+1)}return e.push(d.substring(t,d.length)),e.filter(n=>!!n).map(n=>n.replace(/\\/g,""))}function l(d,e){return i(e).reduce((t,n)=>t?t[n]:null,d)}const y="message",E="response",M="set-interface",v="service-message",C=!!window.MSInputMethodContext&&!!document.documentMode,T={allowedSenderOrigin:void 0};class O{constructor(e,t,n={}){this.remote={},this.serviceMethods={},this.localApi={},this.callbacks={},this._resolveRemoteMethodsPromise=null,this.options=Object.assign(Object.assign({},T),n),this.incrementalID=Math.floor(Math.random()*1e5),this.postMessage=e,this.remoteMethodsWaitPromise=new Promise(r=>{this._resolveRemoteMethodsPromise=r}),t(r=>this.onMessageListener(r))}onMessageListener(e){const t=e.data,{allowedSenderOrigin:n}=this.options;n&&e.origin!==n&&!C||(t.type===E?this.popCallback(t.callId,t.success,t.result):t.type===y?this.callLocalApi(t.methodName,t.arguments).then(r=>this.responseOtherSide(t.callId,r)).catch(r=>this.responseOtherSide(t.callId,r,!1)):t.type===M?(this.setInterface(t.apiMethods),this.responseOtherSide(t.callId)):t.type===v&&this.callLocalServiceMethod(t.methodName,t.arguments).then(r=>this.responseOtherSide(t.callId,r)).catch(r=>this.responseOtherSide(t.callId,r,!1)))}postMessageToOtherSide(e){this.postMessage(e,"*")}setInterface(e){var t;this.remote={},e.forEach(n=>{const r=i(n);let s=this.remote;for(let h=0;h<r.length-1;h++){const f=r[h];(!s[f]||typeof s[f]!="object")&&(s[f]={}),s=s[f]}s[r[r.length-1]]=this.createMethodWrapper(n)}),(t=this._resolveRemoteMethodsPromise)===null||t===void 0||t.call(this)}getMethodsFromInterface(e){return Object.keys(e).reduce((t,n)=>(typeof e[n]=="object"?t.push(...this.getMethodsFromInterface(e[n]).map(r=>`${n}.${r}`)):t.push(n),t),[])}setLocalApi(e){return new Promise((t,n)=>{const r=this.registerCallback(t,n);this.postMessageToOtherSide({callId:r,apiMethods:this.getMethodsFromInterface(e),type:M})}).then(()=>this.localApi=e)}setServiceMethods(e){this.serviceMethods=e}callLocalApi(e,t){const n=l(this.localApi,e);if(!n)throw new Error(`Local method "${e}" is not registered`);return Promise.resolve(n.call(this,...t))}callLocalServiceMethod(e,t){const n=l(this.serviceMethods,e);if(!n)throw new Error(`Service method ${e} is not registered`);return Promise.resolve(n.call(this,...t))}createMethodWrapper(e){return(...t)=>this.callRemoteMethod(e,...t)}callRemoteMethod(e,...t){return new Promise((n,r)=>{const s=this.registerCallback(n,r);this.postMessageToOtherSide({callId:s,methodName:e,type:y,arguments:t})})}callRemoteServiceMethod(e,...t){return new Promise((n,r)=>{const s=this.registerCallback(n,r);this.postMessageToOtherSide({callId:s,methodName:e,type:v,arguments:t})})}responseOtherSide(e,t,n=!0){t instanceof Error&&(t=[...Object.keys(t),"message"].reduce((s,h)=>(s[h]=t[h],s),{}));const r=()=>this.postMessage({callId:e,type:E,success:n,result:t},"*");try{r()}catch(s){console.error("Failed to post response, recovering...",s),s instanceof DOMException&&(t=JSON.parse(JSON.stringify(t)),r())}}registerCallback(e,t){const n=(++this.incrementalID).toString();return this.callbacks[n]={successCallback:e,failureCallback:t},n}popCallback(e,t,n){const r=this.callbacks[e];r&&(t?r.successCallback(n):r.failureCallback(n),delete this.callbacks[e])}}const k=O;var I=a(466),x=a.n(I);const w={frameContainer:"body",frameClassName:"websandbox__frame",frameSrc:null,frameContent:`
<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body></body>
</html>
  `,codeToRunBeforeInit:null,initialStyles:null,baseUrl:null,allowPointerLock:!1,allowFullScreen:!1,sandboxAdditionalAttributes:""};class b{static create(e,t={}){return new b(e,t)}constructor(e,t){this.connection=null,this.removeMessageListener=()=>{},this.validateOptions(t),this.options=Object.assign(Object.assign({},w),t),this.iframe=this.createIframe(),this.promise=new Promise(n=>{this.connection=new k(this.iframe.contentWindow.postMessage.bind(this.iframe.contentWindow),r=>{const s=h=>{if(h.source===this.iframe.contentWindow)return r(h)};window.addEventListener("message",s),this.removeMessageListener=()=>window.removeEventListener("message",s)},{allowedSenderOrigin:"null"}),this.connection.setServiceMethods({iframeInitialized:()=>this.connection.setLocalApi(e).then(()=>n(this))})})}validateOptions(e){var t;if(e.frameSrc&&(e.frameContent||e.initialStyles||e.baseUrl||e.codeToRunBeforeInit))throw new Error('You can not set both "frameSrc" and any of frameContent,initialStyles,baseUrl,codeToRunBeforeInit options');if("frameContent"in e&&!(!((t=e.frameContent)===null||t===void 0)&&t.includes("<head>")))throw new Error('Websandbox: iFrame content must have "<head>" tag.')}_prepareFrameContent(e){var t,n,r;let s=(t=e.frameContent)!==null&&t!==void 0?t:"";return e.codeToRunBeforeInit&&(s=(n=s.replace("<head>",`<head>
<script>${e.codeToRunBeforeInit}<\/script>`))!==null&&n!==void 0?n:""),s=(r=s.replace("<head>",`<head>
<script>${x()}<\/script>`))!==null&&r!==void 0?r:"",e.initialStyles&&(s=s.replace("</head>",`<style>${e.initialStyles}</style>
</head>`)),e.baseUrl&&(s=s.replace("<head>",`<head>
<base target="_parent" href="${e.baseUrl}"/>`)),s}createIframe(){var e;const t=this.options.frameContainer,n=typeof t=="string"?document.querySelector(t):t;if(!n)throw new Error("Websandbox: Cannot find container for sandbox "+n);const r=document.createElement("iframe");return r.sandbox=`allow-scripts ${this.options.sandboxAdditionalAttributes}`,r.allow=`${this.options.allowAdditionalAttributes}`,r.className=(e=this.options.frameClassName)!==null&&e!==void 0?e:"",this.options.allowFullScreen&&(r.allowFullscreen=!0),this.options.frameSrc?(r.src=this.options.frameSrc,n.appendChild(r),r):(r.setAttribute("srcdoc",this._prepareFrameContent(this.options)),n.appendChild(r),r)}destroy(){this.iframe.remove(),this.removeMessageListener()}_runCode(e){return this.connection.callRemoteServiceMethod("runCode",e)}_runFunction(e){return this._runCode(`(${e.toString()})()`)}run(e){return e.name?this._runFunction(e):this._runCode(e)}importScript(e){return this.connection.callRemoteServiceMethod("importScript",e)}injectStyle(e){return this.connection.callRemoteServiceMethod("injectStyle",e)}}const R=b})(),p})())})(S)),S.exports}var P=A();const L=j(P),W=N({__proto__:null,default:L},[P]);export{W as w};
