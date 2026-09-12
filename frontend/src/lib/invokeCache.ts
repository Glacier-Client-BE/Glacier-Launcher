import { invoke } from '@tauri-apps/api/core'

/**
 * Thin TTL cache around `invoke` for read-only lookups that barely change
 * between launches (GitHub release lists, the Windows Update version list).
 * Backed by sessionStorage so a stale value never survives a full app
 * restart — only repeated panel opens within one run benefit.
 */
export async function invokeCached<T>(
  cmd: string,
  args: Record<string, unknown> | undefined,
  ttlMs: number,
): Promise<T> {
  const key = `glacier-cache:${cmd}:${JSON.stringify(args ?? {})}`
  try {
    const raw = sessionStorage.getItem(key)
    if (raw) {
      const { value, at } = JSON.parse(raw) as { value: T; at: number }
      if (Date.now() - at < ttlMs) return value
    }
  } catch {
    // corrupt/unavailable storage — fall through to a live invoke
  }

  const value = await invoke<T>(cmd, args)
  try {
    sessionStorage.setItem(key, JSON.stringify({ value, at: Date.now() }))
  } catch {
    // storage full or unavailable — caching is a bonus, not a requirement
  }
  return value
}
