<script lang="ts">
  import { onMount } from 'svelte';
  import { me, changeEmail, changePassword } from '$lib/api/client';
  import type { MeResponse } from '$lib/types';

  let user = $state<MeResponse | null>(null);

  // Email state
  let editEmail = $state('');
  let editingEmail = $state(false);
  let emailSaving = $state(false);

  // Password state
  let currentPassword = $state('');
  let newPassword = $state('');
  let confirmPassword = $state('');
  let passwordSaving = $state(false);

  let error = $state('');
  let message = $state('');

  onMount(async () => {
    try {
      user = await me();
      editEmail = user.email;
    } catch {
      // handled by layout
    }
  });

  async function handleChangeEmail(e: Event) {
    e.preventDefault();
    error = '';
    message = '';
    const trimmed = editEmail.trim();
    if (!trimmed || !trimmed.includes('@')) {
      error = 'Enter a valid email';
      return;
    }
    emailSaving = true;
    try {
      await changeEmail(trimmed);
      user = await me();
      editingEmail = false;
      message = 'Email updated';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to update email';
    } finally {
      emailSaving = false;
    }
  }

  async function handleChangePassword(e: Event) {
    e.preventDefault();
    error = '';
    message = '';
    if (newPassword.length < 8) {
      error = 'New password must be at least 8 characters';
      return;
    }
    if (newPassword !== confirmPassword) {
      error = 'Passwords do not match';
      return;
    }
    passwordSaving = true;
    try {
      await changePassword(currentPassword, newPassword);
      message = 'Password changed successfully';
      currentPassword = '';
      newPassword = '';
      confirmPassword = '';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to change password';
    } finally {
      passwordSaving = false;
    }
  }
</script>

<div class="page-header">
  <h1>Profile</h1>
</div>

{#if error}
  <p class="error">{error}</p>
{/if}
{#if message}
  <p class="text-success">{message}</p>
{/if}

{#if user}
  <div class="card" style="max-width: 400px; margin-bottom: 20px;">
    <div class="form-group">
      <span class="field-label">Email</span>
      {#if editingEmail}
        <form onsubmit={handleChangeEmail} style="display: flex; gap: 8px; align-items: center;">
          <input type="email" bind:value={editEmail} required style="flex: 1;" />
          <button type="submit" class="compact" disabled={emailSaving}>
            {emailSaving ? 'Saving...' : 'Save'}
          </button>
          <button
            type="button"
            class="alt compact"
            onclick={() => {
              editingEmail = false;
              editEmail = user?.email ?? '';
            }}>Cancel</button
          >
        </form>
      {:else}
        <span
          style="font-size: 14px; color: var(--ink); display: flex; align-items: center; gap: 8px;"
        >
          {user.email}
          <button class="ghost compact" onclick={() => (editingEmail = true)}>Edit</button>
        </span>
      {/if}
    </div>
    <div class="form-group">
      <span class="field-label">Role</span>
      <span class="badge">{user.role}</span>
    </div>
  </div>
{/if}

<h2>Change Password</h2>

<form onsubmit={handleChangePassword} class="card" style="max-width: 400px;">
  <div class="form-group">
    <label for="currentPw">
      Current Password
      <input id="currentPw" type="password" bind:value={currentPassword} required />
    </label>
  </div>
  <div class="form-group">
    <label for="newPw">
      New Password
      <input
        id="newPw"
        type="password"
        bind:value={newPassword}
        required
        minlength={8}
        placeholder="Min 8 characters"
      />
    </label>
  </div>
  <div class="form-group">
    <label for="confirmPw">
      Confirm New Password
      <input id="confirmPw" type="password" bind:value={confirmPassword} required minlength={8} />
    </label>
  </div>
  <button type="submit" disabled={passwordSaving}>
    {passwordSaving ? 'Saving...' : 'Change Password'}
  </button>
</form>
