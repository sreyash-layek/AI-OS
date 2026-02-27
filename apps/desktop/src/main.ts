const promptInput = document.getElementById("prompt") as HTMLInputElement;
const sendBtn = document.getElementById("send") as HTMLButtonElement;
const voiceBtn = document.getElementById("voice") as HTMLButtonElement;
const responseEl = document.getElementById("response") as HTMLPreElement;

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
