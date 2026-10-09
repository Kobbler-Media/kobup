<script lang="ts">
  import { onMount } from 'svelte';
  import { listDevices } from '$lib/api/client';
  import type { Device } from '$lib/types';

  let devices = $state<Device[]>([]);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      devices = await listDevices();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load devices';
    } finally {
      loading = false;
    }
  });
</script>

<div class="page-header">
  <h1>Devices</h1>
  {#if !loading && devices.length > 0}
    <span class="badge">{devices.length} registered</span>
  {/if}
</div>

{#if loading}
  <p class="loading">Loading devices...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if devices.length === 0}
  <div class="empty-state">
    <p>No devices registered yet.</p>
  </div>
{:else}
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Platform</th>
        <th>SDK</th>
        <th>TV</th>
        <th>Kobup Version</th>
        <th>Last Seen</th>
      </tr>
    </thead>
    <tbody>
      {#each devices as device (device.id)}
        <tr>
          <td><strong>{device.deviceName}</strong></td>
          <td>{device.platform}</td>
          <td>{device.sdkInt ?? '-'}</td>
          <td>
            {#if device.isTv}
              <span class="badge warning">TV</span>
            {:else}
              <span class="text-muted">-</span>
            {/if}
          </td>
          <td>{device.kobupVersion ?? '-'}</td>
          <td class="text-muted">{new Date(device.lastSeenAt).toLocaleString()}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .empty-state {
    text-align: center;
    padding: 48px 20px;
    color: var(--muted);
  }
</style>
