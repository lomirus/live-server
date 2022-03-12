
const ws = new WebSocket("ws://{}:{}/live-server-ws");
ws.onopen = () => console.log("[Live Server] Connection Established");
ws.onmessage = () => location.reload();
ws.onclose = () => console.log("[Live Server] Connection Closed");
