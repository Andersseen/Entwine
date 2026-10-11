export const SUMMARY_URI = "entwine://project/summary";
export const DOCUMENT_TEMPLATE = "entwine://document/{id}";

/** Document ids may contain `/` and `:`; they are percent-encoded as one segment. */
export function documentUri(id: string): string {
  return `entwine://document/${encodeURIComponent(id)}`;
}

export function idFromVariable(value: string | string[]): string {
  const raw = Array.isArray(value) ? (value[0] ?? "") : value;
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}
