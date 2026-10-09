<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { me, login, logout as apiLogout } from '$lib/api/client';
  import { ApiError } from '$lib/api/client';
  import '../app.css';

  let { children } = $props();

  // Routes that render standalone, without the admin auth shell or login gate.
  const PUBLIC_PATHS = ['/get'];
  function pathIsPublic(path: string): boolean {
    return PUBLIC_PATHS.some((p) => path === p || path.startsWith(p + '/'));
  }
  let isPublic = $derived(pathIsPublic($page.url.pathname));

  let authed = $state(false);
  let hydrated = $state(false);
  let userEmail = $state('');
  let menuOpen = $state(false);
  let theme = $state<'dark' | 'light'>('dark');

  // Login form state
  let loginEmail = $state('');
  let loginPassword = $state('');
  let loginError = $state('');
  let loginLoading = $state(false);

  const navLinks = [
    {
      href: '/projects',
      label: 'Projects',
      icon: 'M21 7V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v2m18 0v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7m18 0H3m4-2v4m10-4v4'
    },
    {
      href: '/devices',
      label: 'Devices',
      icon: 'M7 2h10v2H7V2zm-2 4h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2zm0 2v10h14V8H5z'
    },
    {
      href: '/tokens',
      label: 'Tokens',
      icon: 'M12.65 10C11.83 7.67 9.61 6 7 6c-3.31 0-6 2.69-6 6s2.69 6 6 6c2.61 0 4.83-1.67 5.65-4H17v4h4v-4h2v-4H12.65zM7 14c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2z'
    },
    {
      href: '/users',
      label: 'Users',
      icon: 'M16 11c1.66 0 3-1.34 3-3s-1.34-3-3-3-3 1.34-3 3 1.34 3 3 3zm-8 0c1.66 0 3-1.34 3-3S9.66 5 8 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 2 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z'
    },
    {
      href: '/pin-login',
      label: 'PIN Login',
      icon: 'M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z'
    }
  ];

  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    localStorage.setItem('kobup-theme', theme);
  }

  function isActive(href: string): boolean {
    return $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
  }

  async function handleLogin(e: Event) {
    e.preventDefault();
    loginError = '';
    loginLoading = true;
    try {
      await login(loginEmail, loginPassword);
      const resp = await me();
      authed = true;
      userEmail = resp.email;
      loginEmail = '';
      loginPassword = '';
      if ($page.url.pathname === '/' || $page.url.pathname === '/login') {
        goto('/projects', { replaceState: true });
      }
    } catch (err) {
      loginError = err instanceof Error ? err.message : 'Login failed';
    } finally {
      loginLoading = false;
    }
  }

  async function handleLogout() {
    try {
      await apiLogout();
    } catch {
      // ignore
    }
    authed = false;
    userEmail = '';
    goto('/login');
  }

  onMount(async () => {
    const stored = localStorage.getItem('kobup-theme');
    if (stored === 'light' || stored === 'dark') {
      theme = stored;
    } else if (window.matchMedia?.('(prefers-color-scheme: light)').matches) {
      theme = 'light';
    }

    try {
      const resp = await me();
      authed = true;
      userEmail = resp.email;
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        if (
          $page.url.pathname !== '/login' &&
          $page.url.pathname !== '/' &&
          !pathIsPublic($page.url.pathname)
        ) {
          goto('/login');
        }
      }
    } finally {
      hydrated = true;
    }
  });
</script>

{#if isPublic}
  <div class="public-shell" data-theme={theme}>
    {@render children()}
  </div>
{:else}
  <div class="app" data-theme={theme}>
    <header class="topbar">
      {#if authed}
        <button
          class="mobile-menu"
          type="button"
          aria-label="Menu"
          onclick={() => (menuOpen = !menuOpen)}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
            <path d="M4 7h16v2H4V7zm0 5h16v2H4v-2zm0 5h16v2H4v-2z"></path>
          </svg>
        </button>
      {/if}
      <a href="/projects" class="brand">Kobup</a>
      <div class="topbar-actions">
        {#if authed}
          <button class="icon-btn" type="button" aria-label="Toggle theme" onclick={toggleTheme}>
            {#if theme === 'dark'}
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                <path
                  d="M12 7c-2.76 0-5 2.24-5 5s2.24 5 5 5 5-2.24 5-5-2.24-5-5-5zM2 13h2c.55 0 1-.45 1-1s-.45-1-1-1H2c-.55 0-1 .45-1 1s.45 1 1 1zm18 0h2c.55 0 1-.45 1-1s-.45-1-1-1h-2c-.55 0-1 .45-1 1s.45 1 1 1zM11 2v2c0 .55.45 1 1 1s1-.45 1-1V2c0-.55-.45-1-1-1s-1 .45-1 1zm0 18v2c0 .55.45 1 1 1s1-.45 1-1v-2c0-.55-.45-1-1-1s-1 .45-1 1zM5.99 4.58a.996.996 0 0 0-1.41 0 .996.996 0 0 0 0 1.41l1.06 1.06c.39.39 1.03.39 1.41 0s.39-1.03 0-1.41L5.99 4.58zm12.37 12.37a.996.996 0 0 0-1.41 0 .996.996 0 0 0 0 1.41l1.06 1.06c.39.39 1.03.39 1.41 0a.996.996 0 0 0 0-1.41l-1.06-1.06zm1.06-10.96a.996.996 0 0 0 0-1.41.996.996 0 0 0-1.41 0l-1.06 1.06c-.39.39-.39 1.03 0 1.41s1.03.39 1.41 0l1.06-1.06zM7.05 18.36a.996.996 0 0 0 0-1.41.996.996 0 0 0-1.41 0l-1.06 1.06c-.39.39-.39 1.03 0 1.41s1.03.39 1.41 0l1.06-1.06z"
                ></path>
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                <path
                  d="M12 3a9 9 0 1 0 9 9c0-.46-.04-.92-.1-1.36a5.389 5.389 0 0 1-4.4 2.26 5.403 5.403 0 0 1-3.14-9.8c-.44-.06-.9-.1-1.36-.1z"
                ></path>
              </svg>
            {/if}
          </button>
        {/if}
      </div>
    </header>

    <div class="app-body">
      {#if authed}
        <aside class="sidebar">
          <nav>
            {#each navLinks as link (link.href)}
              <a href={link.href} class:active={isActive(link.href)}>
                <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                  <path d={link.icon}></path>
                </svg>
                {link.label}
              </a>
            {/each}
          </nav>
          <div class="sidebar-footer">
            <a href="/profile" class="sidebar-email">{userEmail}</a>
            <button class="logout-btn" type="button" onclick={handleLogout} title="Sign out">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
                <path
                  d="M17 7l-1.41 1.41L18.17 11H8v2h10.17l-2.58 2.58L17 17l5-5zM4 5h8V3H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h8v-2H4V5z"
                ></path>
              </svg>
            </button>
          </div>
        </aside>
      {/if}

      <main class="content">
        {#if !hydrated}
          <div class="loading-card">Loading...</div>
        {:else if !authed}
          <div class="login-wrapper">
            <div class="login-card">
              <div class="login-brand">
                <span class="brand-mark">K</span>
                <span class="brand-text">Kobup</span>
              </div>
              <p class="login-subtitle">Sign in to your admin account</p>

              {#if loginError}
                <p class="login-error">{loginError}</p>
              {/if}

              <form onsubmit={handleLogin}>
                <div class="login-field">
                  <label for="login-email">Email</label>
                  <input
                    id="login-email"
                    type="email"
                    bind:value={loginEmail}
                    required
                    placeholder="you@example.com"
                  />
                </div>
                <div class="login-field">
                  <label for="login-password">Password</label>
                  <input
                    id="login-password"
                    type="password"
                    bind:value={loginPassword}
                    required
                    placeholder="Enter password"
                  />
                </div>
                <button type="submit" class="login-btn" disabled={loginLoading}>
                  {loginLoading ? 'Signing in...' : 'Sign In'}
                </button>
              </form>
            </div>
          </div>
        {:else}
          {@render children()}
        {/if}
      </main>
    </div>
  </div>
{/if}

{#if menuOpen}
  <button
    class="mobile-overlay"
    type="button"
    aria-label="Close menu"
    onclick={() => (menuOpen = false)}
  ></button>
  <aside class="mobile-sidebar">
    <nav>
      {#each navLinks as link (link.href)}
        <a href={link.href} class:active={isActive(link.href)} onclick={() => (menuOpen = false)}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
            <path d={link.icon}></path>
          </svg>
          {link.label}
        </a>
      {/each}
    </nav>
    <div class="sidebar-footer">
      <span class="sidebar-email">{userEmail}</span>
    </div>
  </aside>
{/if}

<style>
  /* ===== CSS Variables (global, matching kobbler) ===== */
  :global(:root) {
    --bg: #0f1115;
    --bg-2: #15181e;
    --ink: #f3f1ee;
    --muted: #9da6b0;
    --accent: #d38c5a;
    --accent-dim: rgba(211, 140, 90, 0.15);
    --card: #191d24;
    --ring: #2a3038;
    --radius: 18px;
    --danger: #ef4444;
    --success: #22c55e;
    --warning: #f97316;
    --shadow: 0 18px 40px rgba(5, 7, 10, 0.5);
  }

  :global([data-theme='light']) {
    --bg: #f5f3ef;
    --bg-2: #ece8e1;
    --ink: #1c1814;
    --muted: #6b6560;
    --accent: #c4773d;
    --accent-dim: rgba(196, 119, 61, 0.12);
    --card: #ffffff;
    --ring: #ddd8d0;
    --shadow: 0 18px 40px rgba(28, 24, 20, 0.12);
  }

  /* ===== Global resets ===== */
  :global(*),
  :global(*::before),
  :global(*::after) {
    box-sizing: border-box;
  }

  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  :global(body) {
    font-family: 'Inter', 'Segoe UI', system-ui, sans-serif;
    background-color: var(--bg);
    background: radial-gradient(circle at top, var(--bg-2) 0%, var(--bg) 50%, var(--bg-2) 100%);
    color: var(--ink);
  }

  :global(*) {
    scrollbar-width: thin;
    scrollbar-color: var(--ring) transparent;
  }

  :global(*::-webkit-scrollbar) {
    width: 8px;
    height: 8px;
  }

  :global(*::-webkit-scrollbar-track) {
    background: transparent;
  }

  :global(*::-webkit-scrollbar-thumb) {
    background: var(--ring);
    border-radius: 4px;
  }

  :global(*::-webkit-scrollbar-thumb:hover) {
    background: var(--muted);
  }

  :global(select option) {
    color: #12161d;
    background: #ffffff;
  }

  /* ===== App shell ===== */
  .app {
    width: 100%;
    height: 100vh;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .public-shell {
    width: 100%;
    height: 100vh;
    height: 100dvh;
    overflow-y: auto;
  }

  .app-body {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  /* ===== Topbar ===== */
  .topbar {
    position: sticky;
    top: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 20px;
    height: 56px;
    border-bottom: 1px solid var(--ring);
    background: linear-gradient(180deg, rgba(21, 24, 30, 0.95), rgba(17, 20, 25, 0.85));
    backdrop-filter: blur(6px);
    box-sizing: border-box;
  }

  :global([data-theme='light']) .topbar {
    background: linear-gradient(180deg, rgba(245, 239, 231, 0.95), rgba(239, 230, 219, 0.85));
  }

  .brand {
    font-family: inherit;
    font-size: 24px;
    font-weight: 700;
    letter-spacing: 0.6px;
    white-space: nowrap;
    text-decoration: none;
    color: var(--ink);
    line-height: 32px;
  }

  .brand:hover {
    color: var(--accent);
    text-decoration: none;
  }

  .topbar-actions {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-left: auto;
  }

  .icon-btn {
    background: var(--card);
    border: 1px solid var(--ring);
    border-radius: 12px;
    width: 32px;
    height: 32px;
    cursor: pointer;
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .icon-btn:hover {
    background: var(--ring);
    transform: none;
    filter: none;
  }

  .mobile-menu {
    display: none;
    border: 1px solid var(--ring);
    background: var(--card);
    color: var(--ink);
    border-radius: 12px;
    width: 32px;
    height: 32px;
    cursor: pointer;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  /* ===== Sidebar ===== */
  .sidebar {
    width: 220px;
    flex-shrink: 0;
    border-right: 1px solid var(--ring);
    overflow-y: auto;
    padding: 16px 0;
    display: flex;
    flex-direction: column;
  }

  .sidebar nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px;
    flex: 1;
  }

  .sidebar nav a {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--muted);
    text-decoration: none;
    transition: all 0.15s;
  }

  .sidebar nav a:hover {
    background: var(--card);
    color: var(--ink);
    text-decoration: none;
  }

  .sidebar nav a.active {
    background: var(--accent-dim);
    color: var(--accent);
    font-weight: 600;
  }

  .sidebar-footer {
    padding: 10px 16px;
    border-top: 1px solid var(--ring);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sidebar-email {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    text-decoration: none;
  }

  .sidebar-email:hover {
    color: var(--accent);
    text-decoration: none;
  }

  .logout-btn {
    background: none;
    border: none;
    color: var(--muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
    flex-shrink: 0;
  }

  .logout-btn:hover {
    color: var(--danger);
  }

  /* ===== Mobile ===== */
  @media (max-width: 768px) {
    .sidebar {
      display: none;
    }

    .mobile-menu {
      display: inline-flex;
    }

    .topbar {
      padding: 0 12px;
      height: 48px;
    }

    .brand {
      font-size: 20px;
      line-height: 28px;
    }

    .content {
      padding: 16px;
    }
  }

  .mobile-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 100;
    border: none;
    padding: 0;
    cursor: default;
    width: 100%;
    height: 100%;
  }

  .mobile-sidebar {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    width: 260px;
    background: var(--bg-2);
    border-right: 1px solid var(--ring);
    z-index: 101;
    padding: 16px 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .mobile-sidebar nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px;
    flex: 1;
  }

  .mobile-sidebar nav a {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--muted);
    text-decoration: none;
    transition: all 0.15s;
  }

  .mobile-sidebar nav a:hover {
    background: var(--card);
    color: var(--ink);
    text-decoration: none;
  }

  .mobile-sidebar nav a.active {
    background: var(--accent-dim);
    color: var(--accent);
    font-weight: 600;
  }

  .mobile-sidebar .sidebar-footer {
    padding: 14px 16px;
    border-top: 1px solid var(--ring);
  }

  .mobile-sidebar .sidebar-email {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  /* ===== Loading card ===== */
  .loading-card {
    max-width: 320px;
    margin: 60px auto 0;
    padding: 16px;
    border-radius: var(--radius);
    border: 1px solid var(--ring);
    background: var(--card);
    text-align: center;
  }

  /* ===== Login form (embedded in layout) ===== */
  .login-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    padding: 20px;
  }

  .login-card {
    background: var(--card);
    border: 1px solid var(--ring);
    border-radius: 20px;
    padding: 36px 32px;
    width: 100%;
    max-width: 380px;
    box-shadow: var(--shadow);
  }

  .login-brand {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }

  .brand-mark {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: var(--accent);
    color: #1d130c;
    font-weight: 700;
    font-size: 18px;
  }

  .brand-text {
    font-weight: 700;
    font-size: 20px;
    letter-spacing: -0.02em;
  }

  .login-subtitle {
    color: var(--muted);
    font-size: 13px;
    margin-bottom: 24px;
  }

  .login-error {
    color: var(--danger);
    margin-bottom: 12px;
    font-size: 13px;
    font-weight: 500;
  }

  .login-field {
    margin-bottom: 14px;
  }

  .login-field label {
    display: grid;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
    font-weight: 500;
  }

  .login-field input {
    padding: 8px 10px;
    border-radius: 10px;
    border: 1px solid var(--ring);
    background: transparent;
    color: var(--ink);
    font-size: 13px;
    font-family: inherit;
    width: 100%;
    transition: border-color 0.15s;
  }

  .login-field input:focus {
    border-color: var(--accent);
    outline: none;
  }

  .login-btn {
    width: 100%;
    justify-content: center;
    margin-top: 4px;
    padding: 12px 20px;
    border-radius: 14px;
    background: var(--accent);
    color: #1d130c;
    font-weight: 700;
    font-size: 14px;
    cursor: pointer;
    border: none;
    font-family: inherit;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .login-btn:hover {
    filter: brightness(1.1);
  }

  .login-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    filter: none;
  }
</style>
