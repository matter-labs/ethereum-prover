import init, {
  deserialize_proof_bytes,
  InitOutput,
  SecurityLevel as WasmSecurityLevel,
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

export type ProofSecurity = "security_80" | "security_100";

export type UnifiedVerificationKey = Uint8Array;

export type VerifierArtifactPair = {
  /** Binary setup data that defines the verifier's cryptographic setup. */
  setupBin: Uint8Array;
  /** Binary layout data that defines circuit layout metadata. */
  layoutBin: Uint8Array;
};

type UnifiedVerifierOptions = (
  | { security80: UnifiedVerificationKey; security100?: UnifiedVerificationKey }
  | { security80?: UnifiedVerificationKey; security100: UnifiedVerificationKey }
) & {
  setupBin?: never;
  layoutBin?: never;
  security?: never;
  legacySecurity80?: never;
  legacySecurity100?: never;
};

type LegacySingleVerifierOptions = {
  /** Legacy single artifact pair. Defaults to 80-bit security. */
  setupBin: Uint8Array;
  /** Legacy single artifact pair. Defaults to 80-bit security. */
  layoutBin: Uint8Array;
  /** Security level for the legacy single artifact pair. */
  security?: ProofSecurity;
  security80?: never;
  security100?: never;
  legacySecurity80?: never;
  legacySecurity100?: never;
};

type LegacyPairVerifierOptions = (
  | { legacySecurity80: VerifierArtifactPair; legacySecurity100?: VerifierArtifactPair }
  | { legacySecurity80?: VerifierArtifactPair; legacySecurity100: VerifierArtifactPair }
) & {
  setupBin?: never;
  layoutBin?: never;
  security?: never;
  security80?: never;
  security100?: never;
};

/**
 * Verifier configuration with explicit verification keys.
 *
 * These correspond to the precomputed verifier artifacts used by the
 * Ethereum STF ZK proof system and must match the proof's circuit version.
 */
export type VerifierOptions =
  | UnifiedVerifierOptions
  | LegacySingleVerifierOptions
  | LegacyPairVerifierOptions;

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
  /**
   * Verifies a proof with an explicitly selected security level.
   *
   * This overrides the security level decoded from the proof payload.
   */
  verifyProofWithSecurity: (handle: ProofHandle, security: ProofSecurity) => VerificationResult;
};

let initPromise: Promise<InitOutput> | null = null;

function ensureInit(): Promise<InitOutput> {
  if (!initPromise) {
    initPromise = init();
  }
  return initPromise;
}

function toWasmSecurityLevel(security: ProofSecurity): WasmSecurityLevel {
  switch (security) {
    case "security_80":
      return WasmSecurityLevel.Security80;
    case "security_100":
      return WasmSecurityLevel.Security100;
    default: {
      const unreachable = security satisfies never;
      throw new Error(`unsupported proof security level: ${unreachable}`);
    }
  }
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

  verifyProofWithSecurity(handle: ProofHandle, security: ProofSecurity): VerificationResult {
    return resultFromWasm(
      this.inner.verifyProofWithSecurity(handle, toWasmSecurityLevel(security))
    );
  }

}

function createWasmVerifier(options: VerifierOptions): WasmVerifier {
  if (options.security80 && options.security100) {
    return WasmVerifier.fromKeys(options.security80, options.security100);
  }

  if (options.security80 || options.security100) {
    const security = options.security100 ? "security_100" : "security_80";
    const verificationKey = options.security100 ?? options.security80;
    if (!verificationKey) {
      throw new Error("missing verification key");
    }
    return WasmVerifier.fromKeyForSecurity(
      verificationKey,
      toWasmSecurityLevel(security)
    );
  }

  if (options.setupBin && options.layoutBin) {
    return WasmVerifier.fromLegacyKey(
      options.setupBin,
      options.layoutBin,
      toWasmSecurityLevel(options.security ?? "security_80")
    );
  }

  if (options.legacySecurity80 && options.legacySecurity100) {
    return WasmVerifier.fromLegacyKeys(
      options.legacySecurity80.setupBin,
      options.legacySecurity80.layoutBin,
      options.legacySecurity100.setupBin,
      options.legacySecurity100.layoutBin
    );
  }

  if (options.legacySecurity80 || options.legacySecurity100) {
    const security = options.legacySecurity100 ? "security_100" : "security_80";
    const artifacts = options.legacySecurity100 ?? options.legacySecurity80;
    if (!artifacts) {
      throw new Error("missing legacy verifier artifacts");
    }
    return WasmVerifier.fromLegacyKey(
      artifacts.setupBin,
      artifacts.layoutBin,
      toWasmSecurityLevel(security)
    );
  }

  throw new Error("verifier options must include explicit verification keys");
}

/**
 * Initializes the WASM dependency and creates a Verifier instance.
 * 
 * @param options Verifier configuration with explicit verification keys.
 * @returns A Promise that resolves to a Verifier instance.
 */
export async function createVerifier(options: VerifierOptions): Promise<Verifier> {
  await ensureInit();

  if (!options) {
    throw new Error("verifier options must include explicit verification keys");
  }

  return new VerifierImpl(createWasmVerifier(options));
}
