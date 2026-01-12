<template>
  <div class="page">
    <header class="hero">
      <div class="title-block">
        <p class="eyebrow">Proof Verifier</p>
        <h1>Verify ethproofs in your browser.</h1>
        <p class="subtitle">
          Provide a URL to a base64+gzip+bincode proof blob. We fetch it, unpack it, and verify it
          with the bundled setup/layout artifacts.
        </p>
      </div>
      <div class="orb" aria-hidden="true"></div>
    </header>

    <main class="card">
      <label class="label" for="proof-file">Proof File</label>
      <div class="input-row">
        <input
          id="proof-file"
          class="input file-input"
          type="file"
          accept=".bin"
          @change="onFileChange"
        />
        <button class="button" :disabled="busy" @click="verify">
          {{ busy ? "Verifying..." : "Verify" }}
        </button>
      </div>

      <p class="hint">
        Tip: upload a gzip+bincode proof blob (binary).
      </p>

      <section class="status" :data-state="status.state">
        <div class="status-header">
          <span class="status-pill">{{ status.label }}</span>
          <span class="status-meta">{{ status.meta }}</span>
        </div>
        <div v-if="status.error" class="errors">
          <p class="errors-title">Errors</p>
          <ul>
            <li>{{ status.error }}</li>
          </ul>
        </div>
      </section>
    </main>

    <footer class="footer">
      <p>Built with wasm + Vue + Vite.</p>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { createVerifier, type VerificationResult } from "@matterlabs/ethproofs-airbender-verifier";

const proofFile = ref<File | null>(null);
const busy = ref(false);

const status = ref({
  state: "idle",
  label: "Waiting",
  meta: "No verification run yet.",
  error: null as string | null
});

const verifierPromise = createVerifier();

function setStatus(update: Partial<typeof status.value>) {
  status.value = { ...status.value, ...update };
}

function onFileChange(event: Event) {
  const target = event.target as HTMLInputElement;
  proofFile.value = target.files && target.files[0] ? target.files[0] : null;
}

async function verify() {
  if (!proofFile.value) {
    setStatus({
      state: "error",
      label: "Missing file",
      meta: "Select a proof file to continue.",
      error: "No file selected."
    });
    return;
  }

  busy.value = true;
  setStatus({
    state: "loading",
    label: "Fetching",
    meta: "Downloading proof data...",
    error: null
  });

  try {
    const verifier = await verifierPromise;
    setStatus({
      state: "loading",
      label: "Verifying",
      meta: "Running verifier in WASM...",
      error: null
    });

    const buffer = await proofFile.value.arrayBuffer();
    const handle = verifier.deserializeProofBytes(new Uint8Array(buffer));
    const result: VerificationResult = verifier.verifyProof(handle);

    if (result.success) {
      setStatus({
        state: "success",
        label: "Verified",
        meta: "Proof verified successfully.",
        error: null
      });
    } else {
      setStatus({
        state: "error",
        label: "Invalid proof",
        meta: "Verifier reported errors.",
        error: result.error
      });
    }
  } catch (error) {
    setStatus({
      state: "error",
      label: "Failed",
      meta: "Verification failed.",
      error: error instanceof Error ? error.message : String(error)
    });
  } finally {
    busy.value = false;
  }
}
</script>

<style scoped>
@import url("https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;600&family=JetBrains+Mono:wght@400;600&display=swap");

:global(body) {
  margin: 0;
  font-family: "Space Grotesk", sans-serif;
  background: radial-gradient(circle at top, #f7d9b6, #f4efe5 45%, #d9e4ff 100%);
  color: #1f1d2b;
  min-height: 100vh;
}

:global(code) {
  font-family: "JetBrains Mono", monospace;
}

.page {
  max-width: 960px;
  margin: 0 auto;
  padding: 48px 24px 32px;
  display: flex;
  flex-direction: column;
  gap: 32px;
}

.hero {
  display: grid;
  gap: 24px;
  align-items: center;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
}

.title-block h1 {
  font-size: clamp(2.2rem, 3vw, 3.2rem);
  margin: 8px 0 12px;
  letter-spacing: -0.03em;
}

.eyebrow {
  text-transform: uppercase;
  letter-spacing: 0.2em;
  font-size: 0.75rem;
  opacity: 0.7;
  margin: 0;
}

.subtitle {
  font-size: 1.05rem;
  line-height: 1.6;
  margin: 0;
  max-width: 38ch;
}

.orb {
  width: 180px;
  aspect-ratio: 1;
  border-radius: 50%;
  background: conic-gradient(from 140deg, #ffbb64, #f46d43, #7a63ff, #4dd0e1, #ffbb64);
  filter: blur(0.2px) drop-shadow(0 28px 40px rgba(32, 46, 84, 0.25));
  justify-self: center;
  animation: float 6s ease-in-out infinite;
}

.card {
  background: rgba(255, 255, 255, 0.8);
  border-radius: 24px;
  padding: 28px;
  box-shadow: 0 22px 60px rgba(17, 28, 64, 0.12);
  border: 1px solid rgba(255, 255, 255, 0.6);
  backdrop-filter: blur(16px);
}

.label {
  font-weight: 600;
  font-size: 0.95rem;
}

.input-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 12px;
  margin-top: 10px;
}

.input {
  padding: 14px 16px;
  border-radius: 14px;
  border: 1px solid #d9d3c8;
  font-size: 1rem;
  background: #fffaf4;
  outline: none;
  transition: border 0.2s ease;
}

.file-input::file-selector-button {
  margin-right: 12px;
  border: none;
  border-radius: 10px;
  padding: 8px 14px;
  background: #1f1d2b;
  color: #fff;
  font-weight: 600;
  cursor: pointer;
}

.input:focus {
  border-color: #3b3eff;
  box-shadow: 0 0 0 4px rgba(59, 62, 255, 0.15);
}

.button {
  border: none;
  border-radius: 14px;
  padding: 12px 22px;
  font-size: 1rem;
  font-weight: 600;
  color: white;
  background: linear-gradient(135deg, #3b3eff, #ff7f50);
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.button:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.button:not(:disabled):hover {
  transform: translateY(-1px);
  box-shadow: 0 10px 25px rgba(59, 62, 255, 0.25);
}

.hint {
  font-size: 0.9rem;
  margin: 12px 0 0;
  color: #4a425a;
}

.status {
  margin-top: 20px;
  padding: 18px;
  border-radius: 18px;
  background: rgba(248, 248, 252, 0.9);
  border: 1px solid rgba(206, 202, 214, 0.6);
}

.status[data-state="success"] {
  border-color: rgba(58, 192, 125, 0.6);
  background: rgba(236, 255, 246, 0.9);
}

.status[data-state="error"] {
  border-color: rgba(255, 115, 97, 0.6);
  background: rgba(255, 240, 238, 0.9);
}

.status-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.status-pill {
  padding: 6px 12px;
  border-radius: 999px;
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  background: #1f1d2b;
  color: #fff;
}

.status-meta {
  font-size: 0.95rem;
  color: #4b4455;
}

.errors {
  margin-top: 12px;
  font-size: 0.95rem;
}

.errors-title {
  font-weight: 600;
  margin: 0 0 6px;
}

.errors ul {
  margin: 0;
  padding-left: 20px;
}

.footer {
  font-size: 0.85rem;
  color: #5c5668;
}

@keyframes float {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-10px);
  }
}

@media (max-width: 640px) {
  .input-row {
    grid-template-columns: 1fr;
  }

  .button {
    width: 100%;
  }
}
</style>
