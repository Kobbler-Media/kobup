<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import {
    getProject,
    updateProject,
    deleteProject,
    createChannel,
    deleteChannel,
    listReleases,
    uploadApks,
    publishRelease,
    deleteRelease,
    pushRefresh,
    uploadProjectIcon
  } from '#lib/api/client.js';
  import { goto } from '$app/navigation';
  import type { ProjectDetail, Release } from '#lib/types.js';

  const projectId = $derived(page.params.id as string);

  let project = $state<ProjectDetail | null>(null);
  let releases = $state<Release[]>([]);
  let loading = $state(true);
  let error = $state('');
  let message = $state('');

  // Edit state
  let editName = $state('');
  let editDescription = $state('');
  let editPackageName = $state('');
  let editMaxReleases = $state<number | null>(null);
  let saving = $state(false);

  // Channel state
  let newChannelName = $state('');
  let addingChannel = $state(false);

  // Icon upload state
  let uploadingIcon = $state(false);

  // Upload state
  let uploadFiles = $state<File[]>([]);
  let uploadChannel = $state('');
  let uploadNotes = $state('');
  let uploadPublish = $state(false);
  let uploading = $state(false);

  onMount(() => loadData());

  async function loadData() {
    loading = true;
    error = '';
    try {
      const [p, r] = await Promise.all([getProject(projectId), listReleases(projectId)]);
      project = p;
      releases = r;
      editName = p.name;
      editDescription = p.description;
      editPackageName = p.packageName;
      editMaxReleases = p.maxReleasesPerChannel ?? null;
      if (p.channels.length > 0) {
        uploadChannel = p.channels.find((c) => c.isDefault)?.name || p.channels[0].name;
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load project';
    } finally {
      loading = false;
    }
  }

  async function handleSave() {
    if (!project) return;
    error = '';
    message = '';
    if (editMaxReleases != null && (!Number.isInteger(editMaxReleases) || editMaxReleases < 0)) {
      error = 'Max releases per channel must be a non-negative integer';
      return;
    }
    saving = true;
    try {
      await updateProject(project.id, {
        name: editName,
        description: editDescription || undefined,
        packageName: editPackageName || undefined,
        maxReleasesPerChannel: editMaxReleases
      });
      message = 'Project updated.';
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to update project';
    } finally {
      saving = false;
    }
  }

  async function handleDelete() {
    if (!project || !confirm(`Delete project "${project.name}"?`)) return;
    try {
      await deleteProject(project.id);
      goto('/projects');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to delete project';
    }
  }

  async function handleAddChannel() {
    if (!project || !newChannelName.trim()) return;
    addingChannel = true;
    error = '';
    try {
      await createChannel(project.id, newChannelName.trim());
      newChannelName = '';
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to add channel';
    } finally {
      addingChannel = false;
    }
  }

  async function handleDeleteChannel(channelId: string) {
    if (!project || !confirm('Delete this channel?')) return;
    try {
      await deleteChannel(project.id, channelId);
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to delete channel';
    }
  }

  function handleFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    uploadFiles = Array.from(input.files ?? []);
  }

  async function handleUpload(e: Event) {
    e.preventDefault();
    if (!project || uploadFiles.length === 0 || !uploadChannel) return;
    uploading = true;
    error = '';
    message = '';
    try {
      await uploadApks(
        project.id,
        uploadFiles,
        uploadChannel,
        uploadNotes || undefined,
        uploadPublish || undefined
      );
      message = `${uploadFiles.length} APK(s) uploaded as one release.`;
      uploadFiles = [];
      uploadNotes = '';
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Upload failed';
    } finally {
      uploading = false;
    }
  }

  async function handlePublish(releaseId: string) {
    error = '';
    message = '';
    try {
      await publishRelease(releaseId);
      message = 'Release published.';
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to publish';
    }
  }

  async function handleDeleteRelease(releaseId: string) {
    if (!confirm('Delete this release?')) return;
    error = '';
    try {
      await deleteRelease(releaseId);
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to delete release';
    }
  }

  async function handleIconUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file || !project) return;
    uploadingIcon = true;
    error = '';
    message = '';
    try {
      await uploadProjectIcon(project.id, file);
      message = 'Icon uploaded.';
      await loadData();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Icon upload failed';
    } finally {
      uploadingIcon = false;
      input.value = '';
    }
  }

  async function handlePushRefresh() {
    if (!project) return;
    error = '';
    message = '';
    try {
      await pushRefresh(project.slug);
      message = 'Push refresh sent.';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Push refresh failed';
    }
  }
</script>

{#if loading}
  <p class="loading">Loading project...</p>
{:else if error && !project}
  <p class="error">{error}</p>
{:else if project}
  <div class="page-header">
    <div class="project-header-left">
      {#if project.iconUrl}
        <img src={project.iconUrl} alt="Project icon" class="project-icon" />
      {:else}
        <div class="project-icon-placeholder">
          {project.name.charAt(0).toUpperCase()}
        </div>
      {/if}
      <div>
        <h1>{project.name}</h1>
        <span class="project-slug">{project.slug}</span>
      </div>
      <label class="icon-upload-label">
        {uploadingIcon ? 'Uploading...' : project.iconUrl ? 'Change icon' : 'Add icon'}
        <input
          type="file"
          accept="image/png,image/jpeg"
          onchange={handleIconUpload}
          hidden
          disabled={uploadingIcon}
        />
      </label>
    </div>
    <div class="actions">
      <button class="alt compact" onclick={handlePushRefresh}>Push Refresh</button>
      <button class="danger compact" onclick={handleDelete}>Delete Project</button>
    </div>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if message}
    <p class="success">{message}</p>
  {/if}

  <!-- Channels -->
  <section>
    <h2>Channels</h2>
    {#if project.channels.length > 0}
      <div class="channel-pills">
        {#each project.channels as channel (channel.id)}
          <span class="channel-pill" class:default={channel.isDefault}>
            {channel.name}
            {#if channel.isDefault}
              <span class="channel-default-tag">default</span>
            {:else}
              <button
                class="channel-remove"
                onclick={() => handleDeleteChannel(channel.id)}
                aria-label="Delete channel">x</button
              >
            {/if}
          </span>
        {/each}
      </div>
    {:else}
      <p class="text-muted text-sm">No channels.</p>
    {/if}
    <div class="channel-add">
      <input type="text" bind:value={newChannelName} placeholder="Channel name" />
      <button class="alt compact" onclick={handleAddChannel} disabled={addingChannel}>Add</button>
    </div>
  </section>

  <!-- Upload APK -->
  <section>
    <h2>Upload APK</h2>
    <form onsubmit={handleUpload} class="card upload-form">
      <div class="form-group">
        <label for="apkFile">
          APK Files
          <input id="apkFile" type="file" accept=".apk" multiple onchange={handleFileSelect} />
        </label>
        <p class="text-muted text-sm" style="margin-top: 4px">
          Select every ABI split of one build (plus a universal APK) to publish them together; each
          device downloads the smallest APK it can run.
        </p>
      </div>
      <div class="form-group">
        <label for="uploadChannel">
          Channel
          <select id="uploadChannel" bind:value={uploadChannel}>
            {#each project.channels as channel (channel.id)}
              <option value={channel.name}>{channel.name}</option>
            {/each}
          </select>
        </label>
      </div>
      <div class="form-group">
        <label for="uploadNotes">
          Release Notes
          <textarea id="uploadNotes" bind:value={uploadNotes} placeholder="Optional"></textarea>
        </label>
      </div>
      <div class="form-group checkbox-group">
        <label class="checkbox-label">
          <input id="uploadPublish" type="checkbox" bind:checked={uploadPublish} />
          Publish immediately
        </label>
      </div>
      <button type="submit" disabled={uploading || uploadFiles.length === 0}>
        {uploading ? 'Uploading...' : 'Upload'}
      </button>
    </form>
  </section>

  <!-- Releases -->
  <section>
    <h2>Releases</h2>
    {#if releases.length === 0}
      <p class="text-muted text-sm">No releases yet.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Version</th>
            <th>Code</th>
            <th>Channel</th>
            <th>APKs</th>
            <th>Status</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each releases as release (release.id)}
            <tr>
              <td><strong>{release.versionName}</strong></td>
              <td>{release.versionCode}</td>
              <td>{release.channelId}</td>
              <td class="text-sm">
                {#each release.artifacts as artifact (artifact.id)}
                  <div title={artifact.fileName}>
                    {artifact.isUniversal
                      ? 'universal'
                      : artifact.supportedAbis.join(', ') || 'no native code'}
                    <span class="text-muted">· {(artifact.sizeBytes / 1048576).toFixed(1)} MB</span>
                  </div>
                {/each}
              </td>
              <td>
                {#if release.publishedAt}
                  <span class="badge published">Published</span>
                {:else}
                  <span class="badge draft">Draft</span>
                {/if}
              </td>
              <td class="text-muted">{new Date(release.createdAt).toLocaleDateString()}</td>
              <td>
                <div class="actions">
                  {#if !release.publishedAt}
                    <button class="compact" onclick={() => handlePublish(release.id)}
                      >Publish</button
                    >
                  {/if}
                  <button class="danger compact" onclick={() => handleDeleteRelease(release.id)}
                    >Delete</button
                  >
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <!-- Project Settings -->
  <section>
    <h2>Project Settings</h2>
    <div class="card settings-form">
      <div class="form-group">
        <label for="editName">
          Name
          <input id="editName" type="text" bind:value={editName} />
        </label>
      </div>
      <div class="form-group">
        <label for="editPackageName">
          Package Name
          <input id="editPackageName" type="text" bind:value={editPackageName} />
        </label>
      </div>
      <div class="form-group">
        <label for="editDescription">
          Description
          <textarea id="editDescription" bind:value={editDescription}></textarea>
        </label>
      </div>
      <div class="form-group">
        <label for="editMaxReleases">
          Max releases per channel
          <input
            id="editMaxReleases"
            type="number"
            min="0"
            step="1"
            bind:value={editMaxReleases}
            placeholder="Leave empty to use global default"
          />
        </label>
        <p class="text-muted text-sm" style="margin-top: 4px">
          Older releases beyond this count are pruned hourly. Empty uses the global default.
        </p>
      </div>
      <p class="text-muted text-sm" style="margin-bottom: 12px">Slug: {project.slug}</p>
      <button onclick={handleSave} disabled={saving}>
        {saving ? 'Saving...' : 'Save Changes'}
      </button>
    </div>
  </section>
{/if}

<style>
  .project-header-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .project-icon {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    object-fit: cover;
  }

  .project-icon-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: var(--accent-dim);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 20px;
  }

  .project-slug {
    font-size: 13px;
    color: var(--muted);
  }

  .icon-upload-label {
    display: inline-flex;
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
    padding: 6px 10px;
    border-radius: 8px;
    background: var(--accent-dim);
    font-weight: 500;
    transition: all 0.15s;
  }

  .icon-upload-label:hover {
    filter: brightness(1.1);
  }

  /* Channels */
  .channel-pills {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }

  .channel-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 100px;
    font-size: 13px;
    font-weight: 600;
    background: var(--bg-2);
    border: 1px solid var(--ring);
    color: var(--ink);
  }

  .channel-pill.default {
    border-color: var(--accent);
    background: var(--accent-dim);
    color: var(--accent);
  }

  .channel-default-tag {
    font-size: 10px;
    font-weight: 400;
    opacity: 0.7;
  }

  .channel-remove {
    background: none;
    border: none;
    padding: 0 2px;
    font-size: 12px;
    color: var(--muted);
    cursor: pointer;
    line-height: 1;
  }

  .channel-remove:hover {
    color: var(--danger);
    transform: none;
    filter: none;
  }

  .channel-add {
    display: flex;
    gap: 8px;
    max-width: 320px;
  }

  /* Forms */
  .upload-form,
  .settings-form {
    max-width: 480px;
  }

  .checkbox-group {
    display: flex;
    align-items: center;
  }

  .checkbox-label {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--ink);
    cursor: pointer;
  }
</style>
