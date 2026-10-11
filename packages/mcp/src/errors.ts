/** Failures that are safe and useful to show to an agent. */
export type ErrorCode =
  | "invalid_root"
  | "binary_not_found"
  | "unsupported_platform"
  | "timeout"
  | "cancelled"
  | "cli_failed"
  | "malformed_output"
  | "unsupported_schema"
  | "unknown_document"
  | "invalid_input";

export class EntwineMcpError extends Error {
  readonly code: ErrorCode;
  constructor(code: ErrorCode, message: string) {
    super(message);
    this.name = "EntwineMcpError";
    this.code = code;
  }
}
