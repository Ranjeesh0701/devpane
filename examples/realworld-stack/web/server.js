const http = require("http");

const PORT = process.env.WEB_PORT || 5173;
const API_URL = process.env.API_URL || "http://localhost:8080";

const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/html" });
  res.end(`
    <!DOCTYPE html>
    <html>
      <head>
        <title>Devpane Realtime Stack</title>
        <style>
          body { font-family: monospace; background: #111216; color: #eeeff4; padding: 40px; }
          .card { border: 1px solid #282b37; background: #161820; padding: 20px; border-radius: 8px; max-width: 600px; }
          .status { color: #55d6a2; font-weight: bold; }
        </style>
      </head>
      <body>
        <div class="card">
          <h2>⚡ Devpane Realtime Stack Active</h2>
          <p class="status">● Frontend running on http://localhost:${PORT}</p>
          <p>Connected to backend API: <code>${API_URL}</code></p>
        </div>
      </body>
    </html>
  `);
});

server.listen(PORT, () => {
  console.log(`[web] Frontend UI server listening on http://localhost:${PORT}`);
  console.log(`[web] Linked to API backend at ${API_URL}`);
  console.log(`[web] Ready to accept user browser connections`);
});
