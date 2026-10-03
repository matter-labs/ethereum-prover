import { WasmVerifier, deserialize_proof_bytes } from "../wasm/pkg/proof_verifier_wasm.js";

/** Decoded proof; release it with `free()`. */
export type ProofHandle = { free(): void };

export type VerificationResult = {
  success: boolean;
  error: string | null;
  /** Eight verified words on success; null on failure. */
  publicOutput: Uint32Array | null;
};

export type VerificationKey = Uint8Array;

export type VerifierOptions = {
  /** Trusted EVKEY001 v2 key for the guest and security 100. */
  verificationKey: VerificationKey;
};

export type Verifier = {
  /** Decode a gzip EPROOF01 v2 proof. Throws on invalid input. */
  deserializeProofBytes(proofBytes: Uint8Array): ProofHandle;
  verifyProof(handle: ProofHandle, expectedOutput?: Uint32Array): VerificationResult;
  /** Release the verifier. */
  free(): void;
};

type Proof = ReturnType<typeof deserialize_proof_bytes>;

function failure(error: unknown): VerificationResult {
  return {
    success: false,
    error: error instanceof Error ? error.message : String(error),
    publicOutput: null
  };
}

class Handle implements ProofHandle {
  constructor(public proof?: Proof) {}

  free(): void {
    this.proof?.free();
    this.proof = undefined;
  }
}

class VerifierImpl implements Verifier {
  private inner?: WasmVerifier;

  constructor(key: Uint8Array) {
    this.inner = WasmVerifier.fromKey(key);
  }

  deserializeProofBytes(proofBytes: Uint8Array): ProofHandle {
    if (!this.inner) throw new Error("verifier has been freed");
    return new Handle(deserialize_proof_bytes(proofBytes));
  }

  verifyProof(handle: ProofHandle, expectedOutput?: Uint32Array): VerificationResult {
    if (!this.inner) return failure("verifier has been freed");
    if (!(handle instanceof Handle) || !handle.proof) return failure("proof handle has been freed");
    try {
      const result = this.inner.verifyProof(handle.proof, expectedOutput);
      const output = { success: result.success, error: result.error() ?? null, publicOutput: result.publicOutput ?? null };
      result.free();
      return output;
    } catch (error) {
      return failure(error);
    }
  }

  free(): void {
    this.inner?.free();
    this.inner = undefined;
  }
}

/** Create a verifier for a trusted v3 key. */
export async function createVerifier(options: VerifierOptions): Promise<Verifier> {
  if (!options || "setupBin" in options || "layoutBin" in options) {
    throw new Error("legacy split keys are not supported; supply verificationKey (EVKEY001 v2)");
  }
  if (!(options.verificationKey instanceof Uint8Array)) {
    throw new Error("verificationKey must be a Uint8Array containing an EVKEY001 v2 key");
  }
  return new VerifierImpl(options.verificationKey);
}

/**
 * Synchronous one-shot verification for callers with a `verify_stark(proof, vk)` contract,
 * such as EthProofs. Returns whether the gzip EPROOF01 v2 proof verifies against the
 * EVKEY001 v2 key; malformed keys or proofs throw.
 */
export function verify_stark(proofBytes: Uint8Array, verificationKey: Uint8Array): boolean {
  const verifier = new VerifierImpl(verificationKey);
  try {
    const handle = verifier.deserializeProofBytes(proofBytes);
    try {
      return verifier.verifyProof(handle).success;
    } finally {
      handle.free();
    }
  } finally {
    verifier.free();
  }
}
