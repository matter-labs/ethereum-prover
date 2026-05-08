import init, {
  deserialize_proof_bytes,
  InitOutput,
  WasmVerifier
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

export type VerificationKey = Uint8Array;

type SingleFileVerifierOptions = {
  /** Single-file verification key. Its embedded security level must match verified proofs. */
  verificationKey: VerificationKey;
  setupBin?: never;
  layoutBin?: never;
};

type LegacySingleVerifierOptions = {
  /** Legacy 80-bit setup data for existing split-key deployments. */
  setupBin: Uint8Array;
  /** Legacy 80-bit layout data for existing split-key deployments. */
  layoutBin: Uint8Array;
  verificationKey?: never;
};

/**
 * Verifier configuration with explicit verification keys.
 *
 * These correspond to the precomputed verifier artifacts used by the
 * Ethereum STF ZK proof system and must match the proof's circuit version.
 */
export type VerifierOptions =
  | SingleFileVerifierOptions
  | LegacySingleVerifierOptions;

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

function resultFromWasm(result: unknown): VerificationResult {
  const typed = result as {
    success: boolean;
    error: () => string | null;
  };

  return {
    success: typed.success,
    error: typed.error()
  };
}

class VerifierImpl implements Verifier {
  constructor(private readonly inner: WasmVerifier) {}

  deserializeProofBytes(proofBytes: Uint8Array): ProofHandle {
    return deserialize_proof_bytes(proofBytes);
  }

  verifyProof(handle: ProofHandle): VerificationResult {
    return resultFromWasm(this.inner.verifyProof(handle));
  }
}

function createWasmVerifier(options: VerifierOptions): WasmVerifier {
  if (options.verificationKey) {
    return WasmVerifier.fromKey(options.verificationKey);
  }

  if (options.setupBin && options.layoutBin) {
    return WasmVerifier.fromLegacyKey(options.setupBin, options.layoutBin);
  }

  throw new Error(
    "verifier options must include a verification key or legacy setup/layout artifacts"
  );
}

/**
 * Initializes the WASM dependency and creates a Verifier instance.
 * 
 * @param options Verifier configuration with an explicit key or legacy artifacts.
 * @returns A Promise that resolves to a Verifier instance.
 */
export async function createVerifier(options: VerifierOptions): Promise<Verifier> {
  await ensureInit();

  if (!options) {
    throw new Error(
      "verifier options must include a verification key or legacy setup/layout artifacts"
    );
  }

  return new VerifierImpl(createWasmVerifier(options));
}
