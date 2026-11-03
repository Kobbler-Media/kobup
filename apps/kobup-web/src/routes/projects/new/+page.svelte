<script lang="ts">
  import { goto } from '$app/navigation';
  import { createProject } from '#lib/api/client.js';

  let slug = $state('');
  let name = $state('');
  let packageName = $state('');
  let description = $state('');
  let maxReleases = $state<number | null>(null);
  let error = $state('');
  let submitting = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    let parsedMax: number | undefined;
    if (maxReleases != null) {
      if (!Number.isInteger(maxReleases) || maxReleases < 0) {
        error = 'Max releases per channel must be a non-negative integer';
        return;
      }
      parsedMax = maxReleases;
    }
    submitting = true;
    try {
      const project = await createProject({
        slug,
        name,
        packageName: packageName || undefined,
        description: description || undefined,
        maxReleasesPerChannel: parsedMax
      });
      goto(`/projects/${project.id}`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to create project';
    } finally {
      submitting = false;
    }
  }
</script>

<div class="page-header">
  <h1>New Project</h1>
</div>

{#if error}
  <p class="error">{error}</p>
{/if}

<form onsubmit={handleSubmit} class="new-project-form card">
  <div class="form-group">
    <label for="slug">
      Slug
      <input id="slug" type="text" bind:value={slug} required placeholder="my-app" />
    </label>
  </div>
  <div class="form-group">
    <label for="name">
      Name
      <input id="name" type="text" bind:value={name} required placeholder="My App" />
    </label>
  </div>
  <div class="form-group">
    <label for="packageName">
      Package Name
      <input
        id="packageName"
        type="text"
        bind:value={packageName}
        placeholder="com.example.myapp"
      />
    </label>
  </div>
  <div class="form-group">
    <label for="description">
      Description
      <textarea id="description" bind:value={description} placeholder="Optional project description"
      ></textarea>
    </label>
  </div>
  <div class="form-group">
    <label for="maxReleases">
      Max releases per channel
      <input
        id="maxReleases"
        type="number"
        min="0"
        step="1"
        bind:value={maxReleases}
        placeholder="Leave empty to use global default"
      />
    </label>
    <p class="hint">
      Older releases beyond this count are pruned hourly. Empty uses the global default.
    </p>
  </div>
  <div class="actions">
    <button type="submit" disabled={submitting}>
      {submitting ? 'Creating...' : 'Create Project'}
    </button>
    <a href="/projects" class="btn alt">Cancel</a>
  </div>
</form>

<style>
  .new-project-form {
    max-width: 480px;
  }

  .hint {
    font-size: 12px;
    color: var(--muted);
    margin: 4px 0 0;
  }
</style>
