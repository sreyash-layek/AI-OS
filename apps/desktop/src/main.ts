const promptInput = document.getElementById("prompt") as HTMLInputElement;
const sendBtn = document.getElementById("send") as HTMLButtonElement;
const voiceBtn = document.getElementById("voice") as HTMLButtonElement;
const stopBtn = document.getElementById("stop") as HTMLButtonElement;
const responseEl = document.getElementById("response") as HTMLPreElement;
const statusEl = document.getElementById("status") as HTMLDivElement;
const toolPreviewEl = document.getElementById("tool-preview") as HTMLPreElement;
const eventLogEl = document.getElementById("event-log") as HTMLPreElement;
const autoSpeakEl = document.getElementById("auto-speak") as HTMLInputElement;

async function sendMessage() {
  const message = promptInput.value.trim();
  if (!message) return;

  responseEl.textContent = "Thinking...";

  try {
    const res = await fetch("/v1/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ message })
    });

    const data = await res.json();
    responseEl.textContent = data.reply ?? "No response";

    if (data.tool_preview) {
      toolPreviewEl.textContent = JSON.stringify(data.tool_preview, null, 2);
    }
  } catch (err) {
    responseEl.textContent = `Daemon offline. Start core-daemon first.\n\n${String(err)}`;
  }
}

sendBtn.addEventListener("click", sendMessage);
promptInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") sendMessage();
});
voiceBtn.addEventListener("click", async () => {
  const text = promptInput.value.trim() || "Hello from AI-OS voice pipeline.";

  try {
    const res = await fetch("/v1/speak", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text, voice: "system-default" })
    });
    const data = await res.json();
    responseEl.textContent = `Speak queued (mode=${data.mode}, request_id=${data.request_id})`;
  } catch (err) {
    responseEl.textContent = `Speak request failed: ${String(err)}`;
  }
});

stopBtn.addEventListener("click", async () => {
  try {
    await fetch("/v1/speak/stop", { method: "POST" });
    responseEl.textContent = "Stop requested.";
  } catch (err) {
    responseEl.textContent = `Stop request failed: ${String(err)}`;
  }
});

async function checkHealth() {
  try {
    const res = await fetch("/health");
    if (!res.ok) throw new Error("health endpoint not ok");
    const json = await res.json();
    statusEl.textContent = `Daemon: ${json.status}`;
    statusEl.className = "status ok";
  } catch {
    statusEl.textContent = "Daemon: offline";
    statusEl.className = "status err";
  }
}

const eventLines: string[] = [];

function pushEventLine(line: string) {
  eventLines.unshift(line);
  if (eventLines.length > 8) eventLines.pop();
  eventLogEl.textContent = eventLines.join("\n");
}

function connectEvents() {
  const ws = new WebSocket("ws://localhost:7777/v1/events");

  ws.onmessage = (ev) => {
    try {
      const payload = JSON.parse(String(ev.data));
      pushEventLine(`[${payload.at}] ${payload.event} (${payload.source})`);

      if (payload.event === "heartbeat" || payload.event === "connected") {
        statusEl.textContent = "Daemon: ok (live)";
        statusEl.className = "status ok";
      }
    } catch {
      pushEventLine(String(ev.data));
    }
  };

  ws.onclose = () => {
    pushEventLine("[reconnect] websocket closed, retrying...");
    setTimeout(connectEvents, 2000);
  };
}

async function loadVoiceConfig() {
  try {
    const res = await fetch("/v1/config/voice");
    const cfg = await res.json();
    autoSpeakEl.checked = Boolean(cfg.auto_speak);
  } catch {
    // ignore for now
  }
}

autoSpeakEl.addEventListener("change", async () => {
  try {
    await fetch("/v1/config/voice", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ auto_speak: autoSpeakEl.checked })
    });
    responseEl.textContent = `Auto-speak set to ${autoSpeakEl.checked}.`;
  } catch (err) {
    responseEl.textContent = `Failed to update voice config: ${String(err)}`;
  }
});

checkHealth();
setInterval(checkHealth, 5000);
connectEvents();
loadVoiceConfig();
