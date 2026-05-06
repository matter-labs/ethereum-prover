# Proof Verifier (JS + WASM)

This folder contains a browser-oriented verifier that wraps a Rust WASM verifier.

## Layout

- `proof_verifier_js/wasm`: Rust crate compiled to WASM.
- `proof_verifier_js/ts`: TypeScript wrapper package that bundles the WASM output.

## Build (local)

Build the TypeScript package (this also builds the WASM output into the package):

```sh
cd proof_verifier_js/ts
yarn install
yarn build
```

## Demo app

The demo is a small Vue + Vite app that verifies an uploaded proof in the browser.

```sh
cd proof_verifier_js/demo
yarn install
yarn dev
```

## Usage (browser)

```ts
import { createVerifier } from "@matterlabs/ethproofs-airbender-verifier";

const verifier = await createVerifier();
const proof = verifier.deserializeProofBytes(proofBytes);
const result = verifier.verifyProof(proof);

if (!result.success) {
  console.error(result.errors);
}
```

`createVerifier()` loads bundled 80-bit and 100-bit setup/layout artifacts.
You can override the default artifacts with `createVerifier({ setupBin,
layoutBin })` for legacy 80-bit verification, or with explicit `security80` /
`security100` artifact pairs. Versioned proof payloads route to their declared
security level automatically; legacy payloads are treated as 80-bit proofs.
