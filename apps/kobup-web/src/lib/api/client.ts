import type {
  LoginResponse,
  MeResponse,
  Project,
  ProjectDetail,
  CreateProjectInput,
  UpdateProjectInput,
  Channel,
  Release,
  Device,
  ApiToken,
  User,
  LatestRelease
} from '$lib/types';

const apiBase = import.meta.env.VITE_API_BASE_URL || '';

function buildQueryString(
  params: Record<string, string | number | boolean | undefined | null>
): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      searchParams.set(key, String(value));
    }
  }
  const qs = searchParams.toString();
  return qs ? `?${qs}` : '';
}

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

type FetchOptions = Parameters<typeof fetch>[1];

async function request<T>(path: string, options: FetchOptions = {}): Promise<T> {
  const headers: Record<string, string> = {};
  // Only set Content-Type for non-FormData bodies
  if (options.body && !(options.body instanceof FormData)) {
    headers['Content-Type'] = 'application/json';
  }
  const response = await fetch(`${apiBase}${path}`, {
    ...options,
    credentials: 'include',
    headers: {
      ...headers,
      ...options.headers
    }
  });

  if (!response.ok) {
    let message = `Request failed: ${response.status}`;
    try {
      const error = await response.json();
      if (typeof error?.message === 'string' && error.message.trim()) {
        message = error.message.trim();
      }
    } catch {
      try {
        const text = await response.text();
        if (text.trim()) {
          message = text.trim();
        } else if (response.statusText) {
          message = response.statusText;
        }
      } catch {
        if (response.statusText) {
          message = response.statusText;
        }
      }
    }
    throw new ApiError(response.status, message);
  }

  const contentLength = response.headers.get('content-length');
  if (response.status === 204 || contentLength === '0') {
    return undefined as T;
  }

  const text = await response.text();
  if (!text) {
    return undefined as T;
  }

  return JSON.parse(text);
}

// Auth
export async function login(email: string, password: string): Promise<LoginResponse> {
  return request('/api/v1/admin/login', {
    method: 'POST',
    body: JSON.stringify({ email, password })
  });
}

export async function logout(): Promise<void> {
  return request('/api/v1/admin/logout', { method: 'POST' });
}

export async function me(): Promise<MeResponse> {
  return request('/api/v1/admin/me');
}

export async function changeEmail(email: string): Promise<void> {
  return request('/api/v1/admin/me/email', {
    method: 'PUT',
    body: JSON.stringify({ email })
  });
}

export async function changePassword(currentPassword: string, newPassword: string): Promise<void> {
  return request('/api/v1/admin/me/password', {
    method: 'PUT',
    body: JSON.stringify({ currentPassword, newPassword })
  });
}

export async function resetUserPassword(userId: string, password: string): Promise<void> {
  return request(`/api/v1/admin/users/${userId}/password`, {
    method: 'PUT',
    body: JSON.stringify({ password })
  });
}

// Projects
export async function listProjects(): Promise<Project[]> {
  return request('/api/v1/admin/projects');
}

export async function getProject(id: string): Promise<ProjectDetail> {
  return request(`/api/v1/admin/projects/${id}`);
}

export async function createProject(data: CreateProjectInput): Promise<Project> {
  return request('/api/v1/admin/projects', {
    method: 'POST',
    body: JSON.stringify(data)
  });
}

export async function updateProject(id: string, data: UpdateProjectInput): Promise<Project> {
  return request(`/api/v1/admin/projects/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(data)
  });
}

export async function deleteProject(id: string): Promise<void> {
  return request(`/api/v1/admin/projects/${id}`, { method: 'DELETE' });
}

// Channels
export async function createChannel(projectId: string, name: string): Promise<Channel> {
  return request(`/api/v1/admin/projects/${projectId}/channels`, {
    method: 'POST',
    body: JSON.stringify({ name })
  });
}

export async function deleteChannel(projectId: string, channelId: string): Promise<void> {
  return request(`/api/v1/admin/projects/${projectId}/channels/${channelId}`, {
    method: 'DELETE'
  });
}

// Releases
export async function listReleases(projectId: string): Promise<Release[]> {
  return request(`/api/v1/admin/projects/${projectId}/releases`);
}

export async function uploadApk(
  projectId: string,
  file: File,
  channel: string,
  releaseNotes?: string,
  publish?: boolean
): Promise<Release> {
  const formData = new FormData();
  formData.append('apk', file);
  formData.append('channel', channel);
  if (releaseNotes) formData.append('release_notes', releaseNotes);
  if (publish) formData.append('publish', 'true');
  return request(`/api/v1/admin/projects/${projectId}/upload`, {
    method: 'POST',
    body: formData
  });
}

export async function publishRelease(releaseId: string): Promise<void> {
  return request(`/api/v1/admin/releases/${releaseId}/publish`, { method: 'POST' });
}

export async function deleteRelease(releaseId: string): Promise<void> {
  return request(`/api/v1/admin/releases/${releaseId}`, { method: 'DELETE' });
}

// Devices
export async function listDevices(): Promise<Device[]> {
  return request('/api/v1/admin/devices');
}

// Tokens
export async function listTokens(): Promise<ApiToken[]> {
  return request('/api/v1/admin/tokens');
}

export async function createToken(
  name: string,
  projectId?: string
): Promise<{ token: string; id: string }> {
  return request('/api/v1/admin/tokens', {
    method: 'POST',
    body: JSON.stringify({ name, project_id: projectId })
  });
}

export async function revokeToken(id: string): Promise<void> {
  return request(`/api/v1/admin/tokens/${id}`, { method: 'DELETE' });
}

// Users
export async function listUsers(): Promise<User[]> {
  return request('/api/v1/admin/users');
}

export async function createUser(email: string, password: string, role: string): Promise<User> {
  return request('/api/v1/admin/users', {
    method: 'POST',
    body: JSON.stringify({ email, password, role })
  });
}

export async function deleteUser(id: string): Promise<void> {
  return request(`/api/v1/admin/users/${id}`, { method: 'DELETE' });
}

// PIN login
export async function pinLoginDevice(pin: string, apiUrl: string): Promise<void> {
  return request('/api/v1/admin/pin-login/device', {
    method: 'POST',
    body: JSON.stringify({ pin, api_url: apiUrl })
  });
}

export async function pinLoginCli(pin: string, apiUrl: string): Promise<void> {
  return request('/api/v1/admin/pin-login/cli', {
    method: 'POST',
    body: JSON.stringify({ pin, api_url: apiUrl })
  });
}

// Project icon
export async function uploadProjectIcon(projectId: string, file: File): Promise<void> {
  const response = await fetch(`${apiBase}/api/v1/admin/projects/${projectId}/icon`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'Content-Type': file.type || 'image/png' },
    body: file
  });
  if (!response.ok) {
    const text = await response.text().catch(() => '');
    throw new ApiError(response.status, text || `Upload failed: ${response.status}`);
  }
}

// Public: latest published release for a project (no auth required)
export async function getLatestRelease(slug: string, channel?: string): Promise<LatestRelease> {
  const qs = channel ? buildQueryString({ channel }) : '';
  return request(`/api/v1/projects/${slug}/latest${qs}`);
}

// Push refresh
export async function pushRefresh(projectSlug?: string): Promise<void> {
  const qs = projectSlug ? buildQueryString({ project: projectSlug }) : '';
  return request(`/api/v1/admin/push-refresh${qs}`, { method: 'POST' });
}
