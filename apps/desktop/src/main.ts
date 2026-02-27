const promptInput = document.getElementById("prompt") as HTMLInputElement;
const sendBtn = document.getElementById("send") as HTMLButtonElement;
const voiceBtn = document.getElementById("voice") as HTMLButtonElement;
const responseEl = document.getElementById("response") as HTMLPreElement;
const statusEl = document.getElementById("status") as HTMLDivElement;

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
  } catch (err) {
    responseEl.textContent = `Daemon offline. Start core-daemon first.\n\n${String(err)}`;
  }
}

sendBtn.addEventListener("click", sendMessage);
promptInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") sendMessage();
});
voiceBtn.addEventListener("click", () => {
  responseEl.textContent = "Voice pipeline starts in Sprint 2.";
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

function connectEvents() {
  const ws = new WebSocket("ws://localhost:7777/v1/events");
  ws.onmessage = (ev) => {
    if (String(ev.data).includes("heartbeat")) {
      statusEl.textContent = "Daemon: ok (live)";
      statusEl.className = "status ok";
    }
  };
  ws.onclose = () => {
    setTimeout(connectEvents, 2000);
  };
}

checkHealth();
setInterval(checkHealth, 5000);
connectEvents();
