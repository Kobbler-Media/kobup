import type { Role } from '#lib/types.js';

export const session = $state<{ role: Role | null }>({ role: null });

export function isAdmin(): boolean {
  return session.role === 'admin';
}
