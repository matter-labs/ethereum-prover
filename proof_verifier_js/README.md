# Proof verifier (TypeScript + WASM)

The v3 verifier accepts gzip `EPROOF01` v2 proofs at security 100. Use the trusted
`recursion_unified_v3_security_100.vk.bin` (`EVKEY001` v2) key for the producing
guest. Version 1 and security-80 proofs need the old 0.x package; version 1.0.0
removes legacy split-key options.

- `wasm/`: bounded decoding and unified proof verification.
- `ts/`: the ESM package; the WASM is an ES module import (wasm-pack `bundler` target).
- `demo/`: a Vue app for proof/key uploads and optional expected-output checks.

## Build and test the package

```sh
cd proof_verifier_js/ts
yarn install --frozen-lockfile
yarn build
yarn test /path/to/fixtures
```

The Node test uses the built package and compares all three fixture outputs with
native verification. The [WASM README](wasm/README.md) describes fixture layout
and lower-level tests. Building requires `wasm-pack`, `wasm32-unknown-unknown`,
and the sibling v3 airbender checkout.

## API

```ts
import { createVerifier } from "@matterlabs/ethproofs-airbender-verifier";

const verifier = await createVerifier({ verificationKey });
try {
  const proof = verifier.deserializeProofBytes(proofBytes);
  try {
    const result = verifier.verifyProof(proof); // Optional Uint32Array(8) second argument.
    if (result.success) console.log(result.publicOutput);
    else console.error(result.error);
  } finally {
    proof.free();
  }
} finally {
  verifier.free();
}
```

`publicOutput` contains eight verified u32 words on success and is null on failure.
Decode errors throw; verification errors, including WASM traps, return failure.
All verifiers share one WASM instance. See the [package README](ts/README.md).

## Demo

Build the package first, then install the demo's local package dependency:

```sh
cd proof_verifier_js/demo
yarn install --force
yarn build
yarn dev
```

Upload a `.bin.gz` proof and the v3 key. An optional expected-output field accepts
eight hexadecimal u32 words. Successful verification displays the eight verified
words. After rebuilding the package, repeat `yarn install --force` to refresh the
demo's copied local dependency.
