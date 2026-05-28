import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import ts from "typescript";

const projectRoot = path.resolve(new URL(".", import.meta.url).pathname, "..");
const transportPath = path.join(projectRoot, "src/lib/bridge/transport.ts");
const wryPath = path.join(projectRoot, "src/lib/bridge/wry.ts");

const originalWindow = globalThis.window;
const originalDocument = globalThis.document;

const importTsModule = async (filePath) => {
  const source = await readFile(filePath, "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
    },
    fileName: filePath,
  });

  return import(
    `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`
  );
};

const createWindow = ({ isIframe = false, maenifold } = {}) => {
  const fakeWindow = { maenifold };
  fakeWindow.parent = isIframe ? { host: true } : fakeWindow;
  return fakeWindow;
};

const createDocument = () => ({ body: {}, documentElement: {} });

const setGlobals = (fakeWindow) => {
  globalThis.window = fakeWindow;
  globalThis.document = createDocument();
};

const restoreGlobals = () => {
  if (originalWindow === undefined) {
    delete globalThis.window;
  } else {
    globalThis.window = originalWindow;
  }

  if (originalDocument === undefined) {
    delete globalThis.document;
  } else {
    globalThis.document = originalDocument;
  }
};

try {
  const transportModule = await importTsModule(transportPath);
  const wryModule = await importTsModule(wryPath);
  const { detectTransport } = transportModule;
  const { WryBridge } = wryModule;

  setGlobals(createWindow({ maenifold: {} }));
  assert.equal(detectTransport(), "wry");

  setGlobals(createWindow({ isIframe: true }));
  assert.equal(detectTransport(), "mcp-apps");

  setGlobals(createWindow());
  assert.equal(detectTransport(), "none");

  const invokeCalls = [];
  const invokeShim = {
    async invoke(tool, args) {
      invokeCalls.push({ tool, args });
      return { ok: true, tool, args };
    },
    subscribe() {
      return () => {};
    },
  };

  const invokeBridge = new WryBridge(invokeShim);
  const invokeResult = await invokeBridge.invoke("pipeline_status", {
    id: "P-1",
  });
  assert.deepEqual(invokeCalls, [
    { tool: "pipeline_status", args: { id: "P-1" } },
  ]);
  assert.deepEqual(invokeResult, {
    ok: true,
    tool: "pipeline_status",
    args: { id: "P-1" },
  });

  const subscribers = new Set();
  const subscribeShim = {
    async invoke() {
      return null;
    },
    subscribe(channel, cb) {
      assert.equal(channel, "pipeline-updates");
      subscribers.add(cb);
      return () => subscribers.delete(cb);
    },
  };

  const subscribeBridge = new WryBridge(subscribeShim);
  const received = [];
  const unsubscribe = subscribeBridge.subscribe(
    "pipeline-updates",
    (message) => {
      received.push(message);
    },
  );

  for (const cb of subscribers) {
    cb({ seq: 1 });
  }
  unsubscribe();
  for (const cb of subscribers) {
    cb({ seq: 2 });
  }

  assert.deepEqual(received, [{ seq: 1 }]);

  process.exitCode = 0;
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  restoreGlobals();
}
