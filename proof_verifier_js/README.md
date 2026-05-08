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

const verifier = await createVerifier({
  verificationKey
});
const proof = verifier.deserializeProofBytes(proofBytes);
const result = verifier.verifyProof(proof);

if (!result.success) {
  console.error(result.error);
}
```

`createVerifier()` requires explicit verification keys. New callers should pass
a single-file verification key via `verificationKey`. Existing 80-bit split
setup/layout artifacts can still be passed explicitly with `setupBin` /
`layoutBin`. Versioned proof payloads must match the verification key's declared
security level; legacy payloads are treated as 80-bit proofs.
