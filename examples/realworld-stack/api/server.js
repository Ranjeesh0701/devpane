const http = require("http");

const PORT = process.env.PORT || 8080;
const start = Date.now();
let requestCount = 0;

const server = http.createServer((req, res) => {
  requestCount++;
  const url = req.url;

  if (url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "healthy", uptime: Math.floor((Date.now() - start) / 1000) }));
    return;
  }

  if (url === "/api/stats") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ requests: requestCount, memory: process.memoryUsage() }));
    return;
  }

  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end(`[API Backend] Hello from Devpane! Requests served: ${requestCount}\n`);
});

server.listen(PORT, () => {
  console.log(`[api] Backend HTTP server listening on http://localhost:${PORT}`);
  console.log(`[api] Health check ready at http://localhost:${PORT}/health`);
  console.log(`[api] Environment loaded: NODE_ENV=${process.env.NODE_ENV || "development"}`);
});

// Periodic heartbeat
let tick = 0;
setInterval(() => {
  tick++;
  console.log(`[api] Heartbeat #${tick} • active connections: ${requestCount} • memory: ${(process.memoryUsage().rss / 1024 / 1024).toFixed(1)} MB`);
}, 3000);
