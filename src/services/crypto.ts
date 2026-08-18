import { invoke } from "@tauri-apps/api/core";

export interface CryptoRequest {
  mode: "encrypt" | "decrypt";
  algorithm: string;
  keyId?: string;
  ivId?: string;
  plaintext?: string;
  ciphertext?: string;
}

export interface CryptoResponse {
  ok: boolean;
  result: string;
}

export async function transform(request: CryptoRequest): Promise<string> {
  const response = await invoke<CryptoResponse>("crypto_transform", {
    request: {
      mode: request.mode,
      algorithm: request.algorithm,
      key_id: request.keyId,
      iv_id: request.ivId,
      plaintext: request.plaintext,
      ciphertext: request.ciphertext,
    },
  });
  return response.result;
}
