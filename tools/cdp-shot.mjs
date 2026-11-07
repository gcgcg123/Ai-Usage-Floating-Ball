import fs from "node:fs";

const [targetId, url, widthText, heightText, outputPath, transparentText] =
  process.argv.slice(2);
const width = Number(widthText);
const height = Number(heightText);
const transparent = transparentText === "1";

const pages = await (await fetch("http://127.0.0.1:9333/json")).json();
const page = pages.find((candidate) => candidate.id === targetId);
if (!page) throw new Error(`target ${targetId} not found`);

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.addEventListener("open", resolve, { once: true });
  ws.addEventListener("error", reject, { once: true });
});

let nextId = 1;
const pending = new Map();
ws.addEventListener("message", (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message));
    else resolve(message.result);
  }
});

function command(method, params = {}) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
}

await command("Page.enable");
await command("Emulation.setDeviceMetricsOverride", {
  width,
  height,
  deviceScaleFactor: 1,
  mobile: false,
});
if (transparent) {
  await command("Emulation.setDefaultBackgroundColorOverride", {
    color: { r: 0, g: 0, b: 0, a: 0 },
  });
}
await command("Page.navigate", { url });
await new Promise((resolve) => setTimeout(resolve, 1400));
const screenshot = await command("Page.captureScreenshot", {
  format: "png",
  fromSurface: true,
  captureBeyondViewport: false,
});
fs.writeFileSync(outputPath, Buffer.from(screenshot.data, "base64"));
ws.close();
console.log(`${outputPath}: ${width}x${height}`);
