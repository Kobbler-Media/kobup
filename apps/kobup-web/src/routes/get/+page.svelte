<script lang="ts">
  import { onMount } from 'svelte';
  import QRCode from 'qrcode';
  import { getLatestRelease, ApiError } from '#lib/api/client.js';
  import type { LatestRelease } from '#lib/types.js';

  let release = $state<LatestRelease | null>(null);
  let qrDataUrl = $state('');
  let error = $state('');
  let loading = $state(true);

  function formatSize(bytes: number): string {
    if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
    if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
    if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(0)} KB`;
    return `${bytes} B`;
  }

  onMount(async () => {
    try {
      const latest = await getLatestRelease('kobup');
      release = latest;
      qrDataUrl = await QRCode.toDataURL(latest.downloadUrl, {
        margin: 1,
        width: 220,
        color: { dark: '#1c1814', light: '#ffffff' }
      });
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) {
        error = 'No Kobup release has been published yet.';
      } else {
        error = err instanceof Error ? err.message : 'Failed to load the latest release.';
      }
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>Get Kobup</title>
</svelte:head>

<div class="page">
  <div class="card">
    <div class="brand">
      <span class="brand-mark">K</span>
      <span class="brand-text">Kobup</span>
    </div>
    <p class="tagline">Install the Kobup updater on your Android device</p>

    {#if loading}
      <p class="status">Loading latest release…</p>
    {:else if error}
      <p class="status error">{error}</p>
    {:else if release}
      <div class="version">
        <span class="version-name">v{release.versionName}</span>
        <span class="version-meta">{formatSize(release.sizeBytes)} · APK</span>
      </div>

      <a class="download-btn" href={release.downloadUrl} download>
        <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
          <path d="M5 20h14v-2H5v2zM19 9h-4V3H9v6H5l7 7 7-7z"></path>
        </svg>
        Download APK
      </a>

      {#if qrDataUrl}
        <div class="qr">
          <img src={qrDataUrl} alt="QR code linking to the Kobup APK download" />
          <p class="qr-hint">Scan to download on another device</p>
        </div>
      {/if}

      <p class="note">
        After downloading, open the APK to install. You may need to allow installing apps from your
        browser in Android settings.
      </p>
    {/if}
  </div>
</div>

<style>
  .page {
    min-height: 100vh;
    min-height: 100dvh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    box-sizing: border-box;
  }

  .card {
    background: var(--card);
    border: 1px solid var(--ring);
    border-radius: 20px;
    padding: 40px 32px;
    width: 100%;
    max-width: 380px;
    box-shadow: var(--shadow);
    text-align: center;
  }

  .brand {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-bottom: 8px;
  }

  .brand-mark {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: var(--accent);
    color: #1d130c;
    font-weight: 700;
    font-size: 22px;
  }

  .brand-text {
    font-weight: 700;
    font-size: 26px;
    letter-spacing: -0.02em;
    color: var(--ink);
  }

  .tagline {
    color: var(--muted);
    font-size: 14px;
    margin: 0 0 28px;
  }

  .status {
    color: var(--muted);
    font-size: 14px;
  }

  .status.error {
    color: var(--danger);
  }

  .version {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 20px;
  }

  .version-name {
    font-size: 20px;
    font-weight: 700;
    color: var(--ink);
  }

  .version-meta {
    font-size: 12px;
    color: var(--muted);
  }

  .download-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 14px 20px;
    border-radius: 14px;
    background: var(--accent);
    color: #1d130c;
    font-weight: 700;
    font-size: 15px;
    text-decoration: none;
    box-sizing: border-box;
    transition: filter 0.15s;
  }

  .download-btn:hover {
    filter: brightness(1.1);
    text-decoration: none;
  }

  .qr {
    margin-top: 28px;
  }

  .qr img {
    width: 200px;
    height: 200px;
    border-radius: 12px;
    background: #fff;
    padding: 8px;
    box-sizing: border-box;
  }

  .qr-hint {
    font-size: 12px;
    color: var(--muted);
    margin: 10px 0 0;
  }

  .note {
    margin: 24px 0 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--muted);
  }
</style>
