const [targetId, expression] = process.argv.slice(2);

const pages = await (await fetch("http://127.0.0.1:9222/json")).json();
const page = pages.find((candidate) => candidate.id === targetId);
if (!page) {
  console.error(`target ${targetId} not found`);
  process.exit(2);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.addEventListener("open", resolve, { once: true });
  ws.addEventListener("error", reject, { once: true });
});

const result = await new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error("CDP timeout")), 10_000);
  ws.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id === 1) {
      clearTimeout(timer);
      resolve(message.result);
    }
  });
  ws.send(
    JSON.stringify({
      id: 1,
      method: "Runtime.evaluate",
      params: { expression, returnByValue: true, awaitPromise: true },
    }),
  );
});

ws.close();
console.log(JSON.stringify(result));
