import { invoke } from '@tauri-apps/api/core';

export { invoke };

export class TauriError extends Error {
  constructor(
    message: string,
    public readonly raw?: unknown
  ) {
    super(message);
    this.name = 'TauriError';
  }
}

function extractErrorMessage(e: unknown, command: string): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  if (typeof e === 'object' && e !== null) {
    // Tauri 2 wraps errors as objects, try common keys
    const obj = e as Record<string, unknown>;
    if (typeof obj.message === 'string') return obj.message;
    if (typeof obj.error === 'string') return obj.error;
    if (typeof obj.description === 'string') return obj.description;
    // Serialized AppError enum from Rust
    const keys = Object.keys(obj);
    if (keys.length === 1 && typeof obj[keys[0]] === 'string') {
      return obj[keys[0]] as string;
    }
    try {
      return JSON.stringify(obj);
    } catch {
      // fall through
    }
  }
  return `Command ${command} failed`;
}

export async function safeInvoke<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw new TauriError(extractErrorMessage(e, command), e);
  }
}
