export interface LoginResponse {
  token: string;
  role: string;
  email: string;
}

export interface MeResponse {
  userId: string;
  email: string;
  role: string;
}

export interface Project {
  id: string;
  slug: string;
  name: string;
  description: string;
  packageName: string;
  visibility: string;
  defaultChannel: string;
  sourceType: string;
  iconUrl: string | null;
  maxReleasesPerChannel: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectDetail extends Project {
  channels: Channel[];
}

export interface Channel {
  id: string;
  projectId: string;
  name: string;
  isDefault: boolean;
  createdAt: string;
}

export interface Release {
  id: string;
  projectId: string;
  channelId: string;
  versionCode: number;
  versionName: string;
  releaseNotes: string | null;
  sourceType: string;
  publishedAt: string | null;
  createdAt: string;
  artifacts: Artifact[];
}

export interface Artifact {
  id: string;
  releaseId: string;
  fileName: string;
  packageName: string;
  sha256: string;
  sizeBytes: number;
  versionCode: number;
  versionName: string;
  supportedAbis: string[];
  isUniversal: boolean;
  createdAt: string;
}

export interface Device {
  id: string;
  deviceName: string;
  platform: string;
  sdkInt: number | null;
  abisJson: string;
  isTv: boolean;
  kobupVersion: string | null;
  appVersionCode: number | null;
  lastSeenAt: string;
  createdAt: string;
}

export interface ApiToken {
  id: string;
  projectId: string | null;
  name: string;
  createdAt: string;
  revokedAt: string | null;
}

export interface User {
  id: string;
  email: string;
  role: string;
  createdAt: string;
}

export interface LatestRelease {
  projectSlug: string;
  versionName: string;
  versionCode: number;
  artifactId: string;
  downloadUrl: string;
  sha256: string;
  sizeBytes: number;
  minSdk: number | null;
  reason: string | null;
}

export interface CreateProjectInput {
  slug: string;
  name: string;
  packageName?: string;
  description?: string;
  maxReleasesPerChannel?: number | null;
}

export interface UpdateProjectInput {
  name?: string;
  description?: string;
  packageName?: string;
  maxReleasesPerChannel?: number | null;
}
