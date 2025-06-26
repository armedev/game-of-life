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
  logMessage("!", "WebSocket error", "msg-error"),
);

const canvas = document.getElementById("paint-canvas");
const ctx = canvas.getContext("2d");
const CANVAS_WIDTH = 800;
const CANVAS_HEIGHT = 800;
const GRID_COLS = 100;
const GRID_ROWS = 100;
const CELL_SIZE = CANVAS_WIDTH / GRID_COLS;

// Hover and click state
let hoveredCell = { col: -1, row: -1 };
let cellColors = new Map(); // Store cell colors: "col,row" -> {r, g, b}
let isDragging = false;
let lastDraggedCell = { col: -1, row: -1 };

// Message types
const MESSAGE_TYPES = {
  // sent and received by server
  HELLO: 1,

  // received by server
  CREATE_NEW_GENERATION: 40,
  AWAKEN_RANDOM_CELL: 41,
  KILL_RANDOM_CELL: 42,
  STEP_GENERATION: 43,
  KILL_ALL_CELLS: 45,

  CREATE_NEW_MLP_PAINTING: 20,
  ADVANCE_MLP_PAINTING: 21,

  REQUEST_PIXEL: 200,

  // sent by server
  DRAW_PIXEL: 100,
  DRAW_FRAME: 101,
};

// Canvas interaction handlers
function getCellFromMouseEvent(event) {
  const rect = canvas.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const y = event.clientY - rect.top;

  // Scale coordinates if canvas is displayed at different size
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;

  const canvasX = x * scaleX;
  const canvasY = y * scaleY;

  const col = Math.floor(canvasX / CELL_SIZE);
  const row = Math.floor(canvasY / CELL_SIZE);

  return { col, row };
}

function drawHoverHighlight(col, row) {
  if (col < 0 || col >= GRID_COLS || row < 0 || row >= GRID_ROWS) {
    return;
  }

  ctx.save();
  ctx.fillStyle = "rgba(255, 0, 0, 0.3)"; // Semi-transparent red overlay
  ctx.fillRect(col * CELL_SIZE, row * CELL_SIZE, CELL_SIZE, CELL_SIZE);
  ctx.restore();
}

function clearHoverHighlight(col, row) {
  if (col < 0 || col >= GRID_COLS || row < 0 || row >= GRID_ROWS) {
    return;
  }

  // Clear just this cell area
  ctx.clearRect(col * CELL_SIZE, row * CELL_SIZE, CELL_SIZE, CELL_SIZE);

  // Redraw the cell content if it exists
  redrawCell(col, row);
}

function redrawCell(col, row) {
  const cellKey = `${col},${row}`;
  const color = cellColors.get(cellKey);

  if (color) {
    ctx.fillStyle = `rgb(${color.r},${color.g},${color.b})`;
    ctx.fillRect(col * CELL_SIZE, row * CELL_SIZE, CELL_SIZE, CELL_SIZE);
  }
}

// Mouse event handlers
canvas.addEventListener("mousemove", (event) => {
  const { col, row } = getCellFromMouseEvent(event);

  if (col !== hoveredCell.col || row !== hoveredCell.row) {
    // Clear previous hover highlight
    if (hoveredCell.col >= 0 && hoveredCell.row >= 0) {
      clearHoverHighlight(hoveredCell.col, hoveredCell.row);
    }

    // Update hovered cell
    hoveredCell = { col, row };

    // Draw new hover highlight
    if (col >= 0 && col < GRID_COLS && row >= 0 && row < GRID_ROWS) {
      drawHoverHighlight(col, row);
    }
  }

  // Handle drag events
  if (
    isDragging &&
    col >= 0 &&
    col < GRID_COLS &&
    row >= 0 &&
    row < GRID_ROWS
  ) {
    // Only trigger if we've moved to a different cell
    if (col !== lastDraggedCell.col || row !== lastDraggedCell.row) {
      lastDraggedCell = { col, row };
      onCellClick(col, row); // Trigger the same callback as click
    }
  }
});

canvas.addEventListener("mouseleave", () => {
  // Clear hover highlight when mouse leaves canvas
  if (hoveredCell.col >= 0 && hoveredCell.row >= 0) {
    clearHoverHighlight(hoveredCell.col, hoveredCell.row);
  }
  hoveredCell = { col: -1, row: -1 };
  isDragging = false;
  lastDraggedCell = { col: -1, row: -1 };
});

canvas.addEventListener("mousedown", (event) => {
  const { col, row } = getCellFromMouseEvent(event);

  if (col >= 0 && col < GRID_COLS && row >= 0 && row < GRID_ROWS) {
    isDragging = true;
    lastDraggedCell = { col, row };
    onCellClick(col, row); // Trigger callback for initial click
  }
});

canvas.addEventListener("mouseup", () => {
  isDragging = false;
  lastDraggedCell = { col: -1, row: -1 };
});

// Callback handler for cell clicks - customize this function
function onCellClick(x, y) {
  logMessage(">>", `Cell clicked: (${x}, ${y})`, "msg-out");

  // Add your custom logic here
  // For example, you could send a message to the server:
  const payload = new Uint8Array([x, y]);
  sendMessage(MESSAGE_TYPES.REQUEST_PIXEL, payload);
  logMessage(">>", `Sent pixel: (${x}, ${y})`, "msg-out");
}

socket.addEventListener("message", (event) => {
  const data = new Uint8Array(event.data);
  const msg = decodeMessage(data);

  if (msg.msg_type === MESSAGE_TYPES.DRAW_PIXEL) {
    logMessage("<<", `Received pixel (${msg.payload.length} bytes)`, "msg-in");
    drawCell(msg.payload);
  } else if (msg.msg_type === MESSAGE_TYPES.DRAW_FRAME) {
    logMessage("<<", `Received frame (${msg.payload.length} bytes)`, "msg-in");
    drawFrame(msg.payload);
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
  sendMessage(MESSAGE_TYPES.HELLO, payload);
  logMessage(">>", text, "msg-out");
  input.value = "";
});

const gol = {
  random_generation: () => {
    sendMessage(MESSAGE_TYPES.CREATE_NEW_GENERATION, new Uint8Array());
    logMessage(">>", "GOL: CREATE_NEW_GENERATION", "msg-out");
  },

  awaken_random_cell: () => {
    sendMessage(MESSAGE_TYPES.AWAKEN_RANDOM_CELL, new Uint8Array());
    logMessage(">>", "GOL: AWAKEN_RANDOM_CELL", "msg-out");
  },

  kill_random_cell: () => {
    sendMessage(MESSAGE_TYPES.KILL_RANDOM_CELL, new Uint8Array());
    logMessage(">>", "GOL: KILL_RANDOM_CELL", "msg-out");
  },

  kill_all_cells: () => {
    sendMessage(MESSAGE_TYPES.KILL_ALL_CELLS, new Uint8Array());
    logMessage(">>", "GOL: KILL_ALL_CELLS", "msg-out");
  },

  step_generation: () => {
    sendMessage(MESSAGE_TYPES.STEP_GENERATION, new Uint8Array());
    logMessage(">>", "GOL: STEP_GENERATION", "msg-out");
  },
};

const mlp = {
  create_new_mlp: () => {
    sendMessage(MESSAGE_TYPES.CREATE_NEW_MLP_PAINTING, new Uint8Array());
    logMessage(">>", "MLP: CREATE_NEW_MLP_PAINTING", "msg-out");
  },

  advance_mlp: () => {
    sendMessage(MESSAGE_TYPES.ADVANCE_MLP_PAINTING, new Uint8Array());
    logMessage(">>", "MLP: ADVANCE_MLP_PAINTING", "msg-out");
  },
};

const mapper = {
  n: gol.random_generation,
  a: gol.awaken_random_cell,
  k: gol.kill_random_cell,
  e: gol.kill_all_cells,
  s: gol.step_generation,

  m: mlp.create_new_mlp,
  b: mlp.advance_mlp,

  c: clearCanvas,
};

function clearCanvas() {
  ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
  cellColors.clear();
  hoveredCell = { col: -1, row: -1 };
  drawGridLines();
  logMessage(">>", "Canvas cleared", "msg-out");
}

function isTypingInInput() {
  const activeElement = document.activeElement;
  return (
    activeElement &&
    (activeElement.tagName === "INPUT" ||
      activeElement.tagName === "TEXTAREA" ||
      activeElement.isContentEditable ||
      activeElement.getAttribute("contenteditable") === "true")
  );
}

window.addEventListener("keydown", (e) => {
  if (isTypingInInput()) {
    return;
  }
  mapper[e.key]?.();
});

window.addEventListener("click", (e) => {
  const id = e.target.id;
  mapper[id]?.();
});

function drawCell(payload) {
  if (payload.length !== 7) {
    logMessage(
      "!",
      `Invalid pixel payload size: ${payload.length}`,
      "msg-error",
    );
    return;
  }

  const view = new DataView(payload.buffer, payload.byteOffset);
  const col = view.getUint16(0, false); // big-endian
  const row = view.getUint16(2, false);
  const r = payload[4];
  const g = payload[5];
  const b = payload[6];

  if (col >= GRID_COLS || row >= GRID_ROWS) {
    logMessage("!", `Pixel out of bounds: (${col}, ${row})`, "msg-error");
    return;
  }

  ctx.fillStyle = `rgb(${r},${g},${b})`;
  ctx.fillRect(col * CELL_SIZE, row * CELL_SIZE, CELL_SIZE, CELL_SIZE);

  // Store the cell color
  cellColors.set(`${col},${row}`, { r, g, b });
}

function drawFrame(payload) {
  if (payload.length < 4) {
    logMessage(
      "!",
      `Invalid frame payload size: ${payload.length}`,
      "msg-error",
    );
    return;
  }

  const view = new DataView(payload.buffer, payload.byteOffset);
  const frameWidth = view.getUint16(0, false); // big-endian
  const frameHeight = view.getUint16(2, false);

  const expectedDataSize = frameWidth * frameHeight * 3; // RGB
  const actualDataSize = payload.length - 4; // minus header

  if (actualDataSize !== expectedDataSize) {
    logMessage(
      "!",
      `Frame data size mismatch: expected ${expectedDataSize}, got ${actualDataSize}`,
      "msg-error",
    );
    return;
  }

  if (frameWidth !== GRID_COLS || frameHeight !== GRID_ROWS) {
    logMessage(
      "!",
      `Frame dimensions mismatch: expected ${GRID_COLS}x${GRID_ROWS}, got ${frameWidth}x${frameHeight}`,
      "msg-error",
    );
    return;
  }

  // Clear canvas before drawing frame
  ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
  cellColors.clear();

  // Draw frame data
  const frameData = payload.slice(4); // Skip header
  let dataIndex = 0;

  for (let row = 0; row < frameHeight; row++) {
    for (let col = 0; col < frameWidth; col++) {
      const r = frameData[dataIndex++];
      const g = frameData[dataIndex++];
      const b = frameData[dataIndex++];

      ctx.fillStyle = `rgb(${r},${g},${b})`;
      ctx.fillRect(col * CELL_SIZE, row * CELL_SIZE, CELL_SIZE, CELL_SIZE);

      // Store cell colors (even black cells, in case they're intentional)
      cellColors.set(`${col},${row}`, { r, g, b });
    }
  }

  // Redraw grid lines
  drawGridLines();

  logMessage("<<", `Drew frame: ${frameWidth}x${frameHeight}`, "msg-in");
}

function drawGridLines() {
  return;
  // ctx.strokeStyle = "#eee";
  // ctx.lineWidth = 0.5;
  //
  // for (let x = 0; x <= CANVAS_WIDTH; x += CELL_SIZE) {
  //   ctx.beginPath();
  //   ctx.moveTo(x, 0);
  //   ctx.lineTo(x, CANVAS_HEIGHT);
  //   ctx.stroke();
  // }
  //
  // for (let y = 0; y <= CANVAS_HEIGHT; y += CELL_SIZE) {
  //   ctx.beginPath();
  //   ctx.moveTo(0, y);
  //   ctx.lineTo(CANVAS_WIDTH, y);
  //   ctx.stroke();
  // }
}

// Initialize canvas
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
