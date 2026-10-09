<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  onMount(() => {
    goto('/projects');
  });
</script>

<p class="loading">Redirecting...</p>
