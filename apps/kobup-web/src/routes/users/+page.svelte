<script lang="ts">
  import { onMount } from 'svelte';
  import { listUsers, createUser, deleteUser, resetUserPassword, me } from '#lib/api/client.js';
  import type { User, MeResponse, Role } from '#lib/types.js';

  let users = $state<User[]>([]);
  let loading = $state(true);
  let error = $state('');
  let currentUser = $state<MeResponse | null>(null);

  // Create user state
  let showCreate = $state(false);
  let newEmail = $state('');
  let newPassword = $state('');
  let newRole = $state<Role>('viewer');
  let creating = $state(false);

  onMount(async () => {
    try {
      const [u, m] = await Promise.all([listUsers(), me()]);
      users = u;
      currentUser = m;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load users';
    } finally {
      loading = false;
    }
  });

  async function handleCreate(e: Event) {
    e.preventDefault();
    if (!newEmail.trim() || !newPassword) return;
    creating = true;
    error = '';
    try {
      await createUser(newEmail.trim(), newPassword, newRole);
      newEmail = '';
      newPassword = '';
      newRole = 'viewer';
      showCreate = false;
      users = await listUsers();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to create user';
    } finally {
      creating = false;
    }
  }

  async function handleResetPassword(id: string, email: string) {
    const newPw = prompt(`New password for ${email} (min 8 chars):`);
    if (!newPw || newPw.length < 8) {
      if (newPw !== null) error = 'Password must be at least 8 characters';
      return;
    }
    error = '';
    try {
      await resetUserPassword(id, newPw);
      alert(`Password reset for ${email}`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to reset password';
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Are you sure you want to delete this user?')) return;
    error = '';
    try {
      await deleteUser(id);
      users = await listUsers();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to delete user';
    }
  }
</script>

<div class="page-header">
  <h1>Users</h1>
  {#if !showCreate}
    <button class="icon-btn" title="Add User" onclick={() => (showCreate = true)}>
      <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"
        ><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"></path></svg
      >
    </button>
  {/if}
</div>

{#if error}
  <p class="error">{error}</p>
{/if}

{#if showCreate}
  <form onsubmit={handleCreate} class="card create-form">
    <div class="form-group">
      <label for="userEmail">
        Email
        <input
          id="userEmail"
          type="email"
          bind:value={newEmail}
          required
          placeholder="user@example.com"
        />
      </label>
    </div>
    <div class="form-group">
      <label for="userPassword">
        Password
        <input
          id="userPassword"
          type="password"
          bind:value={newPassword}
          required
          minlength={8}
          placeholder="Min 8 characters"
        />
      </label>
    </div>
    <div class="form-group">
      <label for="userRole">
        Role
        <select id="userRole" bind:value={newRole}>
          <option value="viewer">viewer: browse apps, authorise own devices</option>
          <option value="admin">admin: full access</option>
        </select>
      </label>
    </div>
    <div class="form-actions">
      <button type="submit" disabled={creating}>
        {creating ? 'Creating...' : 'Create'}
      </button>
      <button type="button" class="alt" onclick={() => (showCreate = false)}>Cancel</button>
    </div>
  </form>
{/if}

{#if loading}
  <p class="loading">Loading users...</p>
{:else if users.length === 0}
  <div class="empty-state">
    <p>No users yet.</p>
  </div>
{:else}
  <table>
    <thead>
      <tr>
        <th>Email</th>
        <th>Role</th>
        <th>Created</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      {#each users as user (user.id)}
        <tr>
          <td><strong>{user.email}</strong></td>
          <td>
            <span class="badge">{user.role}</span>
          </td>
          <td class="text-muted">{new Date(user.createdAt).toLocaleDateString()}</td>
          <td>
            {#if currentUser && currentUser.userId !== user.id}
              <div class="form-actions">
                <button class="compact" onclick={() => handleResetPassword(user.id, user.email)}
                  >Reset pw</button
                >
                <button class="danger compact" onclick={() => handleDelete(user.id)}>Delete</button>
              </div>
            {:else}
              <span class="badge success">you</span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .create-form {
    max-width: 400px;
    margin-bottom: 20px;
  }

  .empty-state {
    text-align: center;
    padding: 48px 20px;
    color: var(--muted);
  }
</style>
