const promptInput = document.getElementById("prompt") as HTMLInputElement;
const sendBtn = document.getElementById("send") as HTMLButtonElement;
const voiceBtn = document.getElementById("voice") as HTMLButtonElement;
const stopBtn = document.getElementById("stop") as HTMLButtonElement;
const responseEl = document.getElementById("response") as HTMLPreElement;
const statusEl = document.getElementById("status") as HTMLDivElement;
const speechStateEl = document.getElementById("speech-state") as HTMLDivElement;
const conversationModeEl = document.getElementById("conversation-mode") as HTMLSpanElement;
const conversationLogEl = document.getElementById("conversation-log") as HTMLDivElement;
const micOrbBtn = document.getElementById("mic-orb") as HTMLButtonElement;
const toolPreviewEl = document.getElementById("tool-preview") as HTMLPreElement;
const eventLogEl = document.getElementById("event-log") as HTMLPreElement;
const autoSpeakEl = document.getElementById("auto-speak") as HTMLInputElement;

const scopePathEl = document.getElementById("scope-path") as HTMLInputElement;
const addScopeBtn = document.getElementById("add-scope") as HTMLButtonElement;
const scopesListEl = document.getElementById("scopes-list") as HTMLDivElement;

const searchQueryEl = document.getElementById("search-query") as HTMLInputElement;
const searchBtn = document.getElementById("search-btn") as HTMLButtonElement;
const searchResultsEl = document.getElementById("search-results") as HTMLPreElement;
const batchStatsEl = document.getElementById("batch-stats") as HTMLPreElement;
const batchFreshnessEl = document.getElementById("batch-freshness") as HTMLSpanElement;
const batchCountsEl = document.getElementById("batch-counts") as HTMLDivElement;

function setConversationMode(mode: "idle" | "listening" | "thinking" | "speaking") {
  conversationModeEl.textContent = `Mode: ${mode}`;
}

function addConversationBubble(role: "user" | "assistant", text: string) {
  const bubble = document.createElement("div");
  bubble.className = `bubble ${role}`;
  bubble.textContent = text;
  conversationLogEl.prepend(bubble);

  while (conversationLogEl.children.length > 8) {
    conversationLogEl.removeChild(conversationLogEl.lastChild as ChildNode);
  }
}

async function sendMessage() {
  const message = promptInput.value.trim();
  if (!message) return;

  addConversationBubble("user", message);
  setConversationMode("thinking");
  responseEl.textContent = "Thinking...";

  try {
    const res = await fetch("/v1/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ message })
    });

    const data = await res.json();
    const reply = data.reply ?? "No response";
    responseEl.textContent = reply;
    addConversationBubble("assistant", reply);

    if (data.tool_preview) {
      toolPreviewEl.textContent = JSON.stringify(data.tool_preview, null, 2);
    }

    setConversationMode("idle");
  } catch (err) {
    responseEl.textContent = `Daemon offline. Start core-daemon first.\n\n${String(err)}`;
    setConversationMode("idle");
  }
}

async function loadScopes() {
  try {
    const res = await fetch("/v1/index/scopes");
    const data = await res.json();
    const scopes = (data.scopes ?? []) as Array<{ id: string; path: string; enabled: boolean }>;

    if (!scopes.length) {
      scopesListEl.textContent = "No scopes yet.";
      return;
    }

    scopesListEl.innerHTML = scopes
      .map(
        (s) =>
          `<div class="scope-row"><span title="${s.id}">${s.path} ${s.enabled ? "(watching)" : "(paused)"}</span><div class="scope-actions"><button class="ghost toggle-scope" data-id="${s.id}" data-enabled="${s.enabled}">${s.enabled ? "Pause" : "Resume"}</button><button class="ghost delete-scope" data-id="${s.id}">Remove</button></div></div>`
      )
      .join("");

    scopesListEl.querySelectorAll(".toggle-scope").forEach((el) => {
      el.addEventListener("click", async () => {
        const btn = el as HTMLButtonElement;
        const id = btn.dataset.id;
        const enabled = btn.dataset.enabled === "true";
        if (!id) return;
        await fetch(`/v1/index/scopes/${id}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: !enabled })
        });
        await loadScopes();
      });
    });

    scopesListEl.querySelectorAll(".delete-scope").forEach((el) => {
      el.addEventListener("click", async () => {
        const id = (el as HTMLButtonElement).dataset.id;
        if (!id) return;
        await fetch(`/v1/index/scopes/${id}`, { method: "DELETE" });
        await loadScopes();
      });
    });
  } catch {
    scopesListEl.textContent = "Failed to load scopes.";
  }
}

async function addScope() {
  const path = scopePathEl.value.trim();
  if (!path) return;

  try {
    await fetch("/v1/index/scopes", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, enabled: true })
    });

    scopePathEl.value = "";
    await loadScopes();
  } catch (err) {
    responseEl.textContent = `Failed to add scope: ${String(err)}`;
  }
}

async function runSearch() {
  const q = searchQueryEl.value.trim();
  if (!q) {
    searchResultsEl.textContent = "No search yet.";
    return;
  }

  try {
    const res = await fetch(`/v1/search?q=${encodeURIComponent(q)}`);
    const data = await res.json();
    const results = data.results ?? [];

    if (!results.length) {
      searchResultsEl.textContent = "No results.";
      return;
    }

    searchResultsEl.textContent = JSON.stringify(results, null, 2);
  } catch (err) {
    searchResultsEl.textContent = `Search failed: ${String(err)}`;
  }
}

sendBtn.addEventListener("click", sendMessage);
promptInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") sendMessage();
});

addScopeBtn.addEventListener("click", addScope);
scopePathEl.addEventListener("keydown", (e) => {
  if (e.key === "Enter") addScope();
});

searchBtn.addEventListener("click", runSearch);
searchQueryEl.addEventListener("keydown", (e) => {
  if (e.key === "Enter") runSearch();
});

micOrbBtn?.addEventListener("click", async () => {
  // Voice-first behavior: if already speaking, stop. Otherwise speak prompt (or default line).
  if (!stopBtn.disabled) {
    await fetch("/v1/speak/stop", { method: "POST" });
    setConversationMode("idle");
    return;
  }

  setConversationMode("listening");

  if (promptInput.value.trim()) {
    await sendMessage();
    return;
  }

  const text = "Hey Sreyash, AI-OS is online and ready.";
  try {
    addConversationBubble("assistant", text);
    await fetch("/v1/speak", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text, voice: "system-default" })
    });
  } catch {
    setConversationMode("idle");
  }
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
    if (!data.ok) {
      responseEl.textContent = `Speak rejected (mode=${data.mode})`;
      return;
    }
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
let lastBatchAt: number | null = null;

function pushEventLine(line: string) {
  eventLines.unshift(line);
  if (eventLines.length > 12) eventLines.pop();
  eventLogEl.textContent = eventLines.join("\n");
}

function updateBatchFreshness() {
  if (!lastBatchAt) {
    batchFreshnessEl.textContent = "never";
    return;
  }

  const seconds = Math.max(0, Math.floor((Date.now() - lastBatchAt) / 1000));
  batchFreshnessEl.textContent = `${seconds}s ago`;
}

function setBatchCountChips(create = 0, update = 0, del = 0, rename = 0) {
  batchCountsEl.innerHTML = `
    <span class="chip c">C:${create}</span>
    <span class="chip u">U:${update}</span>
    <span class="chip d">D:${del}</span>
    <span class="chip r">R:${rename}</span>
  `;
}

function connectEvents() {
  const ws = new WebSocket("ws://localhost:7777/v1/events");

  ws.onmessage = (ev) => {
    try {
      const payload = JSON.parse(String(ev.data));
      const suffix = payload?.event?.includes("index") ? " 🧠" : "";
      pushEventLine(`[${payload.at}] ${payload.event} (${payload.source})${suffix}`);

      if (payload.event === "heartbeat" || payload.event === "connected") {
        statusEl.textContent = "Daemon: ok (live)";
        statusEl.className = "status ok";
      }

      if (payload.event === "speech_started") {
        speechStateEl.textContent = "Speaking…";
        speechStateEl.className = "speech-line live";
        voiceBtn.disabled = true;
        stopBtn.disabled = false;
        micOrbBtn?.classList.add("live");
        setConversationMode("speaking");
      }

      if (payload.event === "speech_stopped") {
        const reason = payload?.data?.reason ?? "unknown";
        speechStateEl.textContent = `Idle (${reason})`;
        speechStateEl.className = "speech-line";
        voiceBtn.disabled = false;
        stopBtn.disabled = true;
        micOrbBtn?.classList.remove("live");
        setConversationMode("idle");
      }

      if (
        payload.event === "index_scope_added" ||
        payload.event === "index_scope_removed" ||
        payload.event === "index_scope_updated"
      ) {
        loadScopes();
      }

      if (payload.event === "index_batch_applied") {
        const counts = payload?.data?.counts ?? {};
        const samplePaths = payload?.data?.sample_paths ?? [];
        const scopeId = payload?.data?.scope_id ?? "unknown";
        const createCount = counts.create ?? 0;
        const updateCount = counts.update ?? 0;
        const deleteCount = counts.delete ?? 0;
        const renameCount = counts.rename ?? 0;

        setBatchCountChips(createCount, updateCount, deleteCount, renameCount);
        lastBatchAt = Date.now();
        updateBatchFreshness();

        batchStatsEl.textContent = JSON.stringify(
          {
            scope_id: scopeId,
            counts: {
              create: createCount,
              update: updateCount,
              delete: deleteCount,
              rename: renameCount
            },
            sample_paths: samplePaths
          },
          null,
          2
        );
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
    const [cfgRes, healthRes] = await Promise.all([
      fetch("/v1/config/voice"),
      fetch("/v1/config/voice/health")
    ]);

    const cfg = await cfgRes.json();
    const health = await healthRes.json();

    autoSpeakEl.checked = Boolean(cfg.auto_speak);
    pushEventLine(
      `[voice] configured=${health.configured_provider}, effective=${health.effective_provider}, available=${health.available}`
    );
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

setConversationMode("idle");
checkHealth();
setInterval(checkHealth, 5000);
setBatchCountChips();
updateBatchFreshness();
setInterval(updateBatchFreshness, 1000);
connectEvents();
stopBtn.disabled = true;
loadVoiceConfig();
loadScopes();
