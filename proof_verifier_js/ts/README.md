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

// Create the verifier object
const verifier = await createVerifier();

// Deserialize the submitted proof (without `base64` encoding; e.g. format that is used on EthProofs to store proofs)
const handle = verifier.deserializeProofBytes(proofBytes);
// Verify deserialized proof.
const result = verifier.verifyProof(handle);

if (!result.success) {
  console.error(result.error);
}
```

## Custom setup/layout

Use this when you need to verify proofs against a non-default circuit version.

```ts
import { createVerifier } from "proof-verifier-js";

const verifier = await createVerifier({
  setupBin,
  layoutBin
});
```

`setupBin` is the verifier setup artifact and `layoutBin` is the circuit layout metadata.
Both must match the proof’s circuit version.

## License

MIT or Apache-2.0. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).
