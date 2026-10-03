import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { gzipSync, gunzipSync } from "node:zlib";
import { createVerifier, verify_stark } from "@matterlabs/ethproofs-airbender-verifier";

const fixtures = process.argv[2] ?? process.env.V3_VERIFIER_FIXTURES;
assert(fixtures, "usage: yarn test FIXTURE_DIR (or set V3_VERIFIER_FIXTURES)");
const reference = JSON.parse(readFileSync(new URL("../../wasm/tests/native_reference.json", import.meta.url)));
const key = readFileSync(join(fixtures, "vk/recursion_unified_v3_security_100.vk.bin"));
const blocks = ["26078427", "26078503", "26078715"];
const proofs = Object.fromEntries(blocks.map(block => [block, readFileSync(join(fixtures, "blocks", block, "proof_v2.bin.gz"))]));
let passed = 0;
function check(name, test) {
  test();
  console.log(`PASS ${name}`);
  passed++;
}
function accepted(result, block) {
  assert.equal(result.success, true, result.error);
  assert.equal(result.error, null);
  assert.deepEqual(Array.from(result.publicOutput), reference[block].output.slice(0, 8));
}
function rejected(result, pattern) {
  assert.equal(result.success, false);
  assert.equal(result.publicOutput, null);
  assert.match(result.error, pattern);
}

const copiedKey = Buffer.from(key);
const verifier = await createVerifier({ verificationKey: copiedKey });
// The verifier must not depend on the caller's buffer after construction.
copiedKey.fill(0);
for (const block of blocks) {
  assert.equal(createHash("sha256").update(proofs[block]).digest("hex"), reference[block].proof_sha256);
  const handle = verifier.deserializeProofBytes(proofs[block]);
  const expected = Uint32Array.from(reference[block].output.slice(0, 8));
  check(`${block}: native public output`, () => accepted(verifier.verifyProof(handle), block));
  check(`${block}: matching expected output`, () => accepted(verifier.verifyProof(handle, expected), block));
  expected[0] ^= 1;
  check(`${block}: wrong expected output`, () => rejected(verifier.verifyProof(handle, expected), /public output/));
  check(`${block}: invalid expected-output length`, () => rejected(verifier.verifyProof(handle, new Uint32Array(7)), /exactly 8/));
  const corrupt = Buffer.from(proofs[block]); corrupt[corrupt.length - 1] ^= 1;
  check(`${block}: corrupted gzip`, () => assert.throws(() => verifier.deserializeProofBytes(corrupt), /gzip/));
  check(`${block}: valid after decode failure`, () => accepted(verifier.verifyProof(handle), block));
  handle.free();
  check(`${block}: freed handle`, () => rejected(verifier.verifyProof(handle), /freed/));
  handle.free();
  const tampered = gunzipSync(proofs[block]);
  tampered[tampered.length - 1] ^= 1;
  const tamperedHandle = verifier.deserializeProofBytes(gzipSync(tampered));
  check(`${block}: tampered proof with valid gzip`, () => rejected(verifier.verifyProof(tamperedHandle), /verification failed|unreachable/));
  tamperedHandle.free();
}

// These fixtures encode all 24 key hash words as tag 252 + four little-endian bytes.
const wrongKey = Buffer.from(key);
for (const index of [0, 1, 2]) {
  const offset = 74 + index * 8 * 5;
  assert.equal(wrongKey[offset], 252);
  wrongKey[offset + 1] ^= 1;
}
const wrongVerifier = await createVerifier({ verificationKey: wrongKey });
for (const block of blocks) {
  const handle = wrongVerifier.deserializeProofBytes(proofs[block]);
  check(`${block}: wrong key`, () => rejected(wrongVerifier.verifyProof(handle), /chain/));
  handle.free();
}
wrongVerifier.free();
for (const block of blocks) {
  check(`${block}: verify_stark`, () => assert.equal(verify_stark(proofs[block], key), true));
  check(`${block}: verify_stark wrong key`, () => assert.equal(verify_stark(proofs[block], wrongKey), false));
}

const wrongZeroKey = Buffer.from(key); wrongZeroKey[75] ^= 1;
const shapeVerifier = await createVerifier({ verificationKey: wrongZeroKey });
for (const block of blocks) {
  const handle = shapeVerifier.deserializeProofBytes(proofs[block]);
  check(`${block}: changed hash[0]`, () => {
    const result = shapeVerifier.verifyProof(handle);
    if (block === "26078503") rejected(result, /chain/);
    else accepted(result, block);
  });
  handle.free();
}
shapeVerifier.free();

const independent = await createVerifier({ verificationKey: key });
const otherHandle = independent.deserializeProofBytes(proofs["26078503"]);
check("handle shared between verifiers", () => accepted(verifier.verifyProof(otherHandle), "26078503"));
otherHandle.free();
independent.free();
verifier.free();
verifier.free();
check("freed verifier", () => assert.throws(() => verifier.deserializeProofBytes(proofs["26078503"]), /freed/));
await assert.rejects(createVerifier({ setupBin: key, layoutBin: key }), /legacy split/);
passed++; console.log("PASS legacy split options rejected");
await assert.rejects(createVerifier({}), /verificationKey/);
passed++; console.log("PASS missing key rejected");
console.log(`${passed} package cases passed`);
