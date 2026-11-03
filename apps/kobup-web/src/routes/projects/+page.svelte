<script lang="ts">
  import { onMount } from 'svelte';
  import { listProjects } from '#lib/api/client.js';
  import type { Project } from '#lib/types.js';

  let projects = $state<Project[]>([]);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      projects = await listProjects();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load projects';
    } finally {
      loading = false;
    }
  });
</script>

<div class="page-header">
  <h1>Projects</h1>
  <a href="/projects/new" class="icon-btn" title="New Project">
    <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"
      ><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"></path></svg
    >
  </a>
</div>

{#if loading}
  <p class="loading">Loading projects...</p>
{:else if error}
  <p class="error">{error}</p>
{:else if projects.length === 0}
  <div class="empty-state">
    <p>No projects yet.</p>
    <p style="margin-top: 8px; font-size: 12px;">
      Use <code>kobup init</code> from your Android project, or create one manually above.
    </p>
  </div>
{:else}
  <div class="project-grid">
    {#each projects as project (project.id)}
      <a href="/projects/{project.id}" class="project-card">
        <div class="project-card-header">
          {#if project.iconUrl}
            <img src={project.iconUrl} alt="" class="project-icon" />
          {:else}
            <div class="project-icon-placeholder">
              {project.name.charAt(0).toUpperCase()}
            </div>
          {/if}
          <div class="project-card-title">
            <h3>{project.name}</h3>
            <span class="project-slug">{project.slug}</span>
          </div>
        </div>
        {#if project.packageName}
          <span class="project-package">{project.packageName}</span>
        {/if}
        {#if project.description}
          <p class="project-desc">{project.description}</p>
        {/if}
      </a>
    {/each}
  </div>
{/if}

<style>
  .empty-state {
    text-align: center;
    padding: 48px 20px;
    color: var(--muted);
  }

  .project-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }

  .project-card {
    background: var(--card);
    border: 1px solid var(--ring);
    border-radius: 16px;
    padding: 16px;
    text-decoration: none;
    color: inherit;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .project-card:hover {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    transform: translateY(-2px);
    text-decoration: none;
  }

  .project-card-header {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .project-icon {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .project-icon-placeholder {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background: var(--accent-dim);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 16px;
    flex-shrink: 0;
  }

  .project-card-title {
    min-width: 0;
  }

  .project-card-title h3 {
    margin: 0;
    font-size: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project-slug {
    font-size: 12px;
    color: var(--muted);
  }

  .project-package {
    font-size: 12px;
    color: var(--muted);
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    padding: 4px 8px;
    background: var(--bg-2);
    border-radius: 6px;
    display: inline-block;
    max-width: fit-content;
  }

  .project-desc {
    font-size: 13px;
    color: var(--muted);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
