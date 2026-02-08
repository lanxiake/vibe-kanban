// This file contains TypeScript types for the Server management feature.
// These types mirror the Rust definitions in crates/utils/src/api/servers.rs
// TODO: Once remote:generate-types supports server types, migrate to auto-generated types.

export enum ServerStatus {
  Pending = 'pending',
  Online = 'online',
  Offline = 'offline',
  Error = 'error',
}

export enum ExecutorStatus {
  Unknown = 'unknown',
  Available = 'available',
  Busy = 'busy',
  Error = 'error',
}

export enum SshAuthType {
  Password = 'password',
  Key = 'key',
}

export type Server = {
  id: string;
  organization_id: string;
  name: string;
  description: string | null;
  host: string;
  port: number;
  status: ServerStatus;
  agent_id: string | null;
  agent_version: string | null;
  last_heartbeat_at: string | null;
  connection_error: string | null;
  system_info: SystemInfo | null;
  system_stats: SystemStats | null;
  ssh_enabled: boolean;
  ssh_host: string | null;
  ssh_port: number | null;
  ssh_username: string | null;
  tags: string[];
  created_at: string;
  updated_at: string;
};

export type ServerExecutor = {
  id: string;
  server_id: string;
  executor_type: string;
  executor_name: string | null;
  version: string | null;
  status: ExecutorStatus;
  current_task_id: string | null;
  capabilities: string[];
  config: Record<string, unknown>;
  total_executions: number;
  successful_executions: number;
  failed_executions: number;
  last_used_at: string | null;
  created_at: string;
  updated_at: string;
};

export type SystemInfo = {
  os: string;
  arch: string;
  cpu_cores: number;
  total_memory_gb: number;
  disk_total_gb: number;
  hostname: string;
};

export type SystemStats = {
  cpu_usage_percent: number;
  memory_usage_percent: number;
  memory_available_gb: number;
  disk_usage_percent: number;
  disk_available_gb: number;
  running_tasks: number;
};

export type SshConfig = {
  enabled: boolean;
  host: string | null;
  port: number | null;
  username: string | null;
  auth_type: SshAuthType;
};

export type CreateServerRequest = {
  name: string;
  description: string | null;
  host: string;
  port: number | null;
  tags: string[] | null;
  ssh_config: SshConfig | null;
};

export type CreateServerResponse = {
  server: Server;
  agent_token: string;
  docker_command: string;
};

export type UpdateServerRequest = {
  name: string | null;
  description: string | null;
  host: string | null;
  port: number | null;
  tags: string[] | null;
  ssh_config: SshConfig | null;
};

export type ServerWithExecutors = Server & {
  executors: ServerExecutor[];
};

export type ListServersResponse = {
  servers: ServerWithExecutors[];
  total: number;
};

export type GetServerResponse = {
  server: ServerWithExecutors;
};

export type RegenerateTokenResponse = {
  agent_token: string;
  docker_command: string;
};

export type ExecutorInfo = {
  executor_type: string;
  version: string | null;
  capabilities: string[];
  config_path: string | null;
};
