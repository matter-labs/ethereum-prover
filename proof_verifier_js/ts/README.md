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
  security100: verificationKey100
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
`verifyProof(handle)` automatically routes versioned proof payloads to the
declared security level. Legacy proof payloads do not carry that metadata, so
they are verified as 80-bit proofs. Use `verifyProofWithSecurity(handle,
"security_100")` only when you intentionally want to override the decoded
security level.

## Verification keys

Use single-file verification keys for new integrations:

```ts
import { createVerifier } from "proof-verifier-js";

const verifier = await createVerifier({
  security80: verificationKey80,
  security100: verificationKey100
});
```

Each key must match the proof’s circuit version and security level.

## Legacy setup/layout

Use this only when you need to verify with existing split setup/layout artifacts.

```ts
import { createVerifier } from "proof-verifier-js";

const verifier = await createVerifier({
  setupBin,
  layoutBin
});
```

The legacy `setupBin` / `layoutBin` pair initializes 80-bit verification by
default. To provide legacy split artifacts for both security levels explicitly:

```ts
const verifier = await createVerifier({
  legacySecurity80: { setupBin: setup80, layoutBin: layouts80 },
  legacySecurity100: { setupBin: setup100, layoutBin: layouts100 }
});
```

Each setup/layout pair must match the proof’s circuit version and security
level.

## License

MIT or Apache-2.0. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).
