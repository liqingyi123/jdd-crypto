import CryptoJS from "crypto-js";

const DEFAULT_KEY = "d3YmI1BUOSE2S2YmalBVZUQ=";
const DEFAULT_IV = "0000000000000000";

/** AES-CBC PKCS7 decrypt → UTF-8 string (legacy crypto.js). */
export function aesDecrypt(
  data: string,
  aesKey = DEFAULT_KEY,
  iv = DEFAULT_IV,
): string {
  const key = CryptoJS.enc.Utf8.parse(aesKey);
  const ivParsed = CryptoJS.enc.Utf8.parse(iv);
  return CryptoJS.AES.decrypt(data, key, {
    iv: ivParsed,
    mode: CryptoJS.mode.CBC,
    padding: CryptoJS.pad.Pkcs7,
  }).toString(CryptoJS.enc.Utf8);
}

/**
 * AES-CBC PKCS7 encrypt → URI-encoded Base64 ciphertext.
 * Matches legacy: encodeURI(Base64(ciphertext)).replace(/\+/g, '%2B').
 */
export function aesEncrypt(
  data: string,
  aesKey = DEFAULT_KEY,
  iv = DEFAULT_IV,
): string {
  const key = CryptoJS.enc.Utf8.parse(aesKey);
  const ivParsed = CryptoJS.enc.Utf8.parse(iv);
  const srcs = CryptoJS.enc.Utf8.parse(data);
  const encrypted = CryptoJS.AES.encrypt(srcs, key, {
    iv: ivParsed,
    mode: CryptoJS.mode.CBC,
    padding: CryptoJS.pad.Pkcs7,
  });
  return encodeURI(CryptoJS.enc.Base64.stringify(encrypted.ciphertext)).replace(
    /\+/g,
    "%2B",
  );
}
