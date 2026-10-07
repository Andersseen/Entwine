/** Release targets and the npm platform package each one feeds. */
export interface Target {
  /** Rust target triple. */
  triple: string;
  /** npm `os` value (process.platform). */
  os: "darwin" | "linux" | "win32";
  /** npm `cpu` value (process.arch). */
  cpu: "arm64" | "x64";
}

export const targets: readonly Target[] = [
  { triple: "x86_64-unknown-linux-gnu", os: "linux", cpu: "x64" },
  { triple: "aarch64-apple-darwin", os: "darwin", cpu: "arm64" },
  { triple: "x86_64-apple-darwin", os: "darwin", cpu: "x64" },
  { triple: "x86_64-pc-windows-msvc", os: "win32", cpu: "x64" },
];

export const platformPackage = (target: Target): string =>
  `@entwine/cli-${target.os}-${target.cpu}`;

export function hostTarget(): Target {
  const found = targets.find(
    (t) => t.os === process.platform && t.cpu === process.arch,
  );
  if (!found)
    throw new Error(
      `No release target for ${process.platform}-${process.arch}`,
    );
  return found;
}

/** Cargo writes `entwine.exe` on Windows; callers pass the extension-less path. */
export function binaryPath(
  path: string,
  platform: string = process.platform,
): string {
  return platform === "win32" && !path.toLowerCase().endsWith(".exe")
    ? `${path}.exe`
    : path;
}
