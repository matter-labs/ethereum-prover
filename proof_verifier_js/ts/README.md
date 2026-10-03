# Airbender v3 proof verifier for EthProofs

Version 1.2.0 verifies gzip-compressed `EPROOF01` v2 proofs at security 100.
It ships the WASM verifier as an ES module import (wasm-pack `bundler` target), so
bundlers such as webpack and Turbopack include it in their output, and Node.js loads it
directly (Node 24+).
Supply the trusted `recursion_unified_v3_security_100.vk.bin` key for the producing
Ethereum STF guest. Keys use `EVKEY001` v2 and bind the permitted recursion chains.

Version 1 proofs, security-80 proofs, and split `setupBin` / `layoutBin` keys need
the old 0.x package. This version rejects them and has no split-key options.

## Usage

```ts
import { createVerifier } from "@matterlabs/ethproofs-airbender-verifier";

const verifier = await createVerifier({ verificationKey });
try {
  const handle = verifier.deserializeProofBytes(proofBytes);
  try {
    // Optionally pass eight expected u32 words as a Uint32Array.
    const result = verifier.verifyProof(handle, expectedOutput);
    if (result.success) console.log(result.publicOutput);
    else console.error(result.error);
  } finally {
    handle.free();
  }
} finally {
  verifier.free();
}
```

For a `verify_stark(proof, vk)` contract, such as EthProofs, use the synchronous
one-shot helper. It returns whether the proof verifies; malformed keys or proofs throw.

```ts
import { verify_stark } from "@matterlabs/ethproofs-airbender-verifier";

const isValid = verify_stark(proofBytes, verificationKey);
```

`createVerifier({verificationKey})` is asynchronous; `deserializeProofBytes` and
`verifyProof` remain synchronous. `verifyProof(handle, expectedOutput?)` returns
`{ success, error, publicOutput }`. On success, `error` is null and `publicOutput`
is a copied `Uint32Array` of eight verified words. On failure, `publicOutput` is
null and `error` explains the failure. A supplied expected output must contain
exactly eight words and match the verified output.

Key and decode errors throw. Verification errors, including WASM traps, return
failure. All verifiers share one WASM instance, like the other EthProofs verifiers.

Call `handle.free()` when finished; a second free is a no-op. `verifier.free()`
releases the verifier; handles stay usable with other verifiers.

The decoder limits compressed and decompressed proofs to 64 MiB each, proof
vectors to 16M words, and keys to 194 bytes. It rejects trailing bytes and words.
The eight output words are returned as u32 values, without interpreting them as
a block hash; callers can enforce the application's expected output explicitly.

## Local build and tests

```sh
yarn install --frozen-lockfile
yarn build
yarn test /path/to/fixtures
```

Builds require the repository Rust toolchain, the `wasm32-unknown-unknown` target,
`wasm-pack`, and the sibling v3 airbender checkout. The package test loads its
built public entry point and checks the three real proofs (26078427, 26078503,
26078715) against native outputs, wrong keys, corruption, and output checks. See the WASM crate's tests in [matter-labs/ethereum-prover](https://github.com/matter-labs/ethereum-prover/tree/main/proof_verifier_js/wasm) for
the fixture layout and native reference provenance.

## License

MIT or Apache-2.0. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).
