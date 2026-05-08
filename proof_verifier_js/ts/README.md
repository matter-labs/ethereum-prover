# Airbender ZK Proof Verifier for EthProofs

[Airbender](https://github.com/matter-labs/zksync-airbender) verifier for Ethereum STF ZK proofs submitted to the EthProofs website.
This package bundles the WASM verifier and a small TypeScript wrapper.

## Installation

```sh
yarn add @matterlabs/ethproofs-airbender-verifier
```

## Usage

```ts
import { createVerifier } from "@matterlabs/ethproofs-airbender-verifier";

const verifier = await createVerifier({
  verificationKey
});

// Deserialize the submitted proof (without `base64` encoding; e.g. format that is used on EthProofs to store proofs)
const handle = verifier.deserializeProofBytes(proofBytes);
// Verify deserialized proof.
const result = verifier.verifyProof(handle);

if (!result.success) {
  console.error(result.error);
}
```

`createVerifier()` requires explicit verification keys.
`verifyProof(handle)` requires the proof security level to match the supplied
verification key. Legacy proof payloads do not carry that metadata, so they are
verified as 80-bit proofs.

## Verification keys

Use single-file verification keys for new integrations:

```ts
import { createVerifier } from "proof-verifier-js";

const verifier = await createVerifier({
  verificationKey
});
```

The key must match the proof’s circuit version and security level.

## Legacy setup/layout

Use this only when you need to verify with existing 80-bit split setup/layout
artifacts.

```ts
import { createVerifier } from "proof-verifier-js";

const verifier = await createVerifier({
  setupBin,
  layoutBin
});
```

The legacy `setupBin` / `layoutBin` pair initializes 80-bit verification only.
Use the single-file VK format for 100-bit verification.

## License

MIT or Apache-2.0. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).
