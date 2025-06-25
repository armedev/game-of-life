const socket = new WebSocket("ws://localhost:8080/ws");
socket.binaryType = "arraybuffer";

const logMessage = (prefix, text, className = "") => {
  const container = document.getElementById("log");
  const entry = document.createElement("div");
  entry.textContent = `${prefix} ${text}`;
  entry.className = className;
  container.appendChild(entry);
  container.scrollTop = container.scrollHeight;
};

socket.addEventListener("open", () =>
  logMessage("✓", "WebSocket connected", "msg-in"),
);
socket.addEventListener("close", () =>
  logMessage("×", "WebSocket closed", "msg-in"),
);
socket.addEventListener("error", () =>
  logMessage("!", "WebSocket error", "msg-in"),
);

const canvas = document.getElementById("paint-canvas");
const ctx = canvas.getContext("2d");

const CANVAS_WIDTH = 400;
const CANVAS_HEIGHT = 400;
const GRID_COLS = 40;
const GRID_ROWS = 40;
const CELL_SIZE = CANVAS_WIDTH / GRID_COLS;

socket.addEventListener("message", (event) => {
  const data = new Uint8Array(event.data);
  const msg = decodeMessage(data);

  if (msg.msg_type === 100) {
    drawCell(msg.payload);
  } else {
    const text = new TextDecoder().decode(msg.payload);
    logMessage("<<", text, "msg-in");
  }
});

document.getElementById("msg-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const input = document.getElementById("msg-input");
  const text = input.value;
  const payload = new TextEncoder().encode(text);
  sendMessage(1, payload); // regular text message
  logMessage(">>", text, "msg-out");
  input.value = "";
});

window.addEventListener("keydown", (e) => {
  if (e.key === "p") {
    sendMessage(42, new Uint8Array()); // request cell paint
  }
});

function drawCell(payload) {
  if (payload.length !== 7) return;

  const view = new DataView(payload.buffer);
  const col = view.getUint16(0, false); // big-endian
  const row = view.getUint16(2, false);
  const r = payload[4];
  const g = payload[5];
  const b = payload[6];

  if (col >= GRID_COLS || row >= GRID_ROWS) return;

  ctx.fillStyle = `rgb(${r},${g},${b})`;
  ctx.fillRect(col * CELL_SIZE, row * CELL_SIZE, CELL_SIZE, CELL_SIZE);
}

function drawGridLines() {
  ctx.strokeStyle = "#eee";
  for (let x = 0; x <= CANVAS_WIDTH; x += CELL_SIZE) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, CANVAS_HEIGHT);
    ctx.stroke();
  }
  for (let y = 0; y <= CANVAS_HEIGHT; y += CELL_SIZE) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(CANVAS_WIDTH, y);
    ctx.stroke();
  }
}
drawGridLines();

// === Protocol encoding/decoding ===

function encodeMessage(msgType, flags, payload) {
  const version = 1;
  const length = payload.length;
  const buffer = new Uint8Array(7 + length);
  buffer[0] = version;
  buffer[1] = msgType;
  buffer[2] = flags;
  const view = new DataView(buffer.buffer);
  view.setUint32(3, length, false); // big endian
  buffer.set(payload, 7);
  return buffer;
}

function decodeMessage(data) {
  if (data.length < 7) return {};
  const version = data[0];
  const msgType = data[1];
  const flags = data[2];
  const length = new DataView(data.buffer).getUint32(3, false);
  const payload = data.slice(7, 7 + length);
  return { version, msg_type: msgType, flags, payload };
}

function sendMessage(msgType, payload) {
  const flags = 0x01 | 0x04; // FLAG_START | FLAG_END
  const msg = encodeMessage(msgType, flags, payload);
  socket.send(msg);
}
