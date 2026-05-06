import init, {
  deserialize_proof_bytes,
  init_defaults,
  init_with_all,
  init_with_security,
  InitOutput,
  SecurityLevel as WasmSecurityLevel,
  verify_proof,
  verify_proof_with_security
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

export type VerifierArtifactPair = {
  /** Binary setup data that defines the verifier's cryptographic setup. */
  setupBin: Uint8Array;
  /** Binary layout data that defines circuit layout metadata. */
  layoutBin: Uint8Array;
};

/**
 * Optional verifier configuration for custom setup and layout for circuits.
 *
 * These correspond to the precomputed verifier artifacts used by the
 * Ethereum STF ZK proof system and must match the proof's circuit version.
 */
export type VerifierOptions = {
  /** Legacy single artifact pair. Defaults to 80-bit security. */
  setupBin?: Uint8Array;
  /** Legacy single artifact pair. Defaults to 80-bit security. */
  layoutBin?: Uint8Array;
  /** Security level for the legacy single artifact pair. */
  security?: ProofSecurity;
  /** 80-bit verifier artifacts. */
  security80?: VerifierArtifactPair;
  /** 100-bit verifier artifacts. */
  security100?: VerifierArtifactPair;
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
  deserializeProofBytes(proofBytes: Uint8Array): ProofHandle {
    return deserialize_proof_bytes(proofBytes);
  }

  verifyProof(handle: ProofHandle): VerificationResult {
    return resultFromWasm(verify_proof(handle));
  }

  verifyProofWithSecurity(handle: ProofHandle, security: ProofSecurity): VerificationResult {
    return resultFromWasm(verify_proof_with_security(handle, toWasmSecurityLevel(security)));
  }

}

function initCustomVerifier(options: VerifierOptions): void {
  if (options.security80 && options.security100) {
    init_with_all(
      options.security80.setupBin,
      options.security80.layoutBin,
      options.security100.setupBin,
      options.security100.layoutBin
    );
    return;
  }

  if (options.security80 || options.security100) {
    const security = options.security100 ? "security_100" : "security_80";
    const artifacts = options.security100 ?? options.security80;
    if (!artifacts) {
      throw new Error("missing verifier artifacts");
    }
    init_with_security(
      artifacts.setupBin,
      artifacts.layoutBin,
      toWasmSecurityLevel(security)
    );
    return;
  }

  if (options.setupBin && options.layoutBin) {
    init_with_security(
      options.setupBin,
      options.layoutBin,
      toWasmSecurityLevel(options.security ?? "security_80")
    );
    return;
  }

  throw new Error("custom verifier options must include setup/layout artifacts");
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
    initCustomVerifier(options);
  } else {
    init_defaults();
  }

  return new VerifierImpl();
}
