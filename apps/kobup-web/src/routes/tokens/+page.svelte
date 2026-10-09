<script lang="ts">
  import { onMount } from 'svelte';
  import { listTokens, createToken, revokeToken, listProjects } from '$lib/api/client';
  import type { ApiToken, Project } from '$lib/types';

  let tokens = $state<ApiToken[]>([]);
  let projects = $state<Project[]>([]);
  let loading = $state(true);
  let error = $state('');

  // Create token state
  let showCreate = $state(false);
  let tokenName = $state('');
  let tokenProjectId = $state('');
  let creating = $state(false);
  let createdToken = $state('');

  onMount(async () => {
    try {
      const [t, p] = await Promise.all([listTokens(), listProjects()]);
      tokens = t;
      projects = p;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load tokens';
    } finally {
      loading = false;
    }
  });

  async function handleCreate(e: Event) {
    e.preventDefault();
    if (!tokenName.trim()) return;
    creating = true;
    error = '';
    createdToken = '';
    try {
      const result = await createToken(tokenName.trim(), tokenProjectId || undefined);
      createdToken = result.token;
      tokenName = '';
      tokenProjectId = '';
      tokens = await listTokens();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to create token';
    } finally {
      creating = false;
    }
  }

  async function handleRevoke(id: string) {
    if (!confirm('Revoke this token?')) return;
    error = '';
    try {
      await revokeToken(id);
      tokens = await listTokens();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to revoke token';
    }
  }
</script>

<div class="page-header">
  <h1>API Tokens</h1>
  {#if !showCreate}
    <button class="icon-btn" title="Create Token" onclick={() => (showCreate = true)}>
      <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"
        ><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"></path></svg
      >
    </button>
  {/if}
</div>

{#if error}
  <p class="error">{error}</p>
{/if}

{#if createdToken}
  <div class="card token-created-card">
    <p><strong>Token created! Copy it now -- it won't be shown again.</strong></p>
    <div class="token-display">{createdToken}</div>
    <button class="alt compact" onclick={() => (createdToken = '')}>Dismiss</button>
  </div>
{/if}

{#if showCreate}
  <form onsubmit={handleCreate} class="card create-form">
    <div class="form-group">
      <label for="tokenName">
        Name
        <input id="tokenName" type="text" bind:value={tokenName} required placeholder="ci-deploy" />
      </label>
    </div>
    <div class="form-group">
      <label for="tokenProject">
        Project (optional)
        <select id="tokenProject" bind:value={tokenProjectId}>
          <option value="">All projects</option>
          {#each projects as project (project.id)}
            <option value={project.id}>{project.name}</option>
          {/each}
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
  <p class="loading">Loading tokens...</p>
{:else if tokens.length === 0}
  <div class="empty-state">
    <p>No tokens yet.</p>
  </div>
{:else}
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Project</th>
        <th>Created</th>
        <th>Status</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      {#each tokens as token (token.id)}
        <tr>
          <td><strong>{token.name}</strong></td>
          <td class="text-muted">{token.projectId ?? 'All'}</td>
          <td class="text-muted">{new Date(token.createdAt).toLocaleDateString()}</td>
          <td>
            {#if token.revokedAt}
              <span class="badge danger">Revoked</span>
            {:else}
              <span class="badge success">Active</span>
            {/if}
          </td>
          <td>
            {#if !token.revokedAt}
              <button class="danger compact" onclick={() => handleRevoke(token.id)}>Revoke</button>
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

  .token-created-card {
    margin-bottom: 20px;
    border-color: var(--accent);
  }

  .empty-state {
    text-align: center;
    padding: 48px 20px;
    color: var(--muted);
  }
</style>
