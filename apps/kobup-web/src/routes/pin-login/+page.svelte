<script lang="ts">
  import { pinLoginDevice, pinLoginCli } from '#lib/api/client.js';

  let loginType = $state<'device' | 'cli'>('device');
  let pin = $state('');
  let apiUrl = $state(typeof window !== 'undefined' ? window.location.origin : '');
  let error = $state('');
  let message = $state('');
  let submitting = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    message = '';
    submitting = true;
    try {
      if (loginType === 'device') {
        await pinLoginDevice(pin.toUpperCase(), apiUrl);
        message = 'Device logged in successfully.';
      } else {
        await pinLoginCli(pin.toUpperCase(), apiUrl);
        message = 'CLI logged in successfully.';
      }
      pin = '';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Login failed';
    } finally {
      submitting = false;
    }
  }
</script>

<div class="page-header">
  <h1>PIN Login</h1>
</div>

<form onsubmit={handleSubmit} class="card pin-form">
  <p class="hint">Enter the PIN shown on a device or CLI to authorize it with this server.</p>

  <div class="type-toggle">
    <button
      type="button"
      class="toggle-btn"
      class:active={loginType === 'device'}
      onclick={() => (loginType = 'device')}
    >
      Device
    </button>
    <button
      type="button"
      class="toggle-btn"
      class:active={loginType === 'cli'}
      onclick={() => (loginType = 'cli')}
    >
      CLI
    </button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if message}
    <p class="success-msg">{message}</p>
  {/if}

  <div class="form-group">
    <label for="pin">
      PIN
      <input
        id="pin"
        type="text"
        bind:value={pin}
        required
        maxlength="32"
        placeholder="K7M2P9XRJ4"
        class="pin-input"
        autocomplete="off"
      />
    </label>
  </div>

  <div class="form-group">
    <label for="apiUrl">
      API URL
      <input id="apiUrl" type="url" bind:value={apiUrl} required />
    </label>
  </div>

  <button type="submit" disabled={submitting}>
    {submitting ? 'Authorizing...' : loginType === 'device' ? 'Authorize Device' : 'Authorize CLI'}
  </button>
</form>

<style>
  .pin-form {
    max-width: 420px;
  }

  .hint {
    font-size: 13px;
    color: var(--muted);
    line-height: 1.5;
    margin-bottom: 4px;
  }

  .type-toggle {
    display: flex;
    gap: 0;
    border: 1px solid var(--ring);
    border-radius: 10px;
    overflow: hidden;
  }

  .toggle-btn {
    flex: 1;
    padding: 8px 16px;
    background: transparent;
    border: none;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .toggle-btn.active {
    background: var(--accent-dim);
    color: var(--accent);
  }

  .toggle-btn:not(.active):hover {
    background: var(--bg-2);
  }

  .pin-input {
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-size: 1.25rem;
    max-width: 280px;
    text-align: center;
    font-weight: 700;
  }

  .success-msg {
    color: var(--success);
    font-size: 13px;
    font-weight: 600;
  }
</style>
