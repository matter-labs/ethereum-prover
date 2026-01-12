import init, {
  deserialize_proof_bytes,
  init_defaults,
  init_with,
  InitOutput,
  verify_proof
} from "../wasm/pkg/proof_verifier_wasm";

/**
 * Opaque handle returned after deserializing a proof blob for verification.
 */
export type ProofHandle = ReturnType<typeof deserialize_proof_bytes>;

/**
 * Result of a proof verification run.
 */
export type VerificationResult = {
  /** True if the proof is valid. */
  success: boolean;
  /** Error details reported by the verifier, or null on success. */
  error: string | null;
};

/**
 * Optional verifier configuration for custom setup and layout for circuits.
 *
 * These correspond to the precomputed verifier artifacts used by the
 * Ethereum STF ZK proof system and must match the proof's circuit version.
 */
export type VerifierOptions = {
  /** Binary setup data that defines the verifier's cryptographic setup. */
  setupBin: Uint8Array;
  /** Binary layout data that defines circuit layout metadata. */
  layoutBin: Uint8Array;
};

/**
 * Verifier API for Ethereum STF ZK proofs submitted to EthProofs.
 */
export type Verifier = {
  /**
   * Deserializes a proof into an internal handle suitable for verification.
   * 
   * @param proofBytes Raw proof bytes as submitted to EthProofs.
   * @returns ProofHandle for use in verifyProof.
   */
  deserializeProofBytes: (proofBytes: Uint8Array) => ProofHandle;
  /**
   * Verifies a previously deserialized proof handle.
   * 
   * @param handle ProofHandle obtained from deserializeProofBytes.
   * @returns VerificationResult describing success/failure.
   */
  verifyProof: (handle: ProofHandle) => VerificationResult;
};

let initPromise: Promise<InitOutput> | null = null;

function ensureInit(): Promise<InitOutput> {
  if (!initPromise) {
    initPromise = init();
  }
  return initPromise;
}

class VerifierImpl implements Verifier {
  deserializeProofBytes(proofBytes: Uint8Array): ProofHandle {
    return deserialize_proof_bytes(proofBytes);
  }

  verifyProof(handle: ProofHandle): VerificationResult {
    const result = verify_proof(handle) as unknown as {
      success: boolean;
      error: () => string | null;
    };

    return {
      success: result.success,
      error: result.error()
    };
  }

}

/**
 * Initializes the WASM dependency and creates a Verifier instance.
 * 
 * @param options Optional verifier configuration for custom setup and layout for circuits.
 * @returns A Promise that resolves to a Verifier instance.
 */
export async function createVerifier(options?: VerifierOptions): Promise<Verifier> {
  await ensureInit();

  if (options) {
    init_with(options.setupBin, options.layoutBin);
  } else {
    init_defaults();
  }

  return new VerifierImpl();
}
