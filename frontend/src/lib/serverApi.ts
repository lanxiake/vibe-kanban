/**
 * Server 管理 API 调用层
 *
 * 提供服务器 CRUD、Token 重新生成等 API 函数，
 * 基于 remoteApi.ts 中的 makeRequest 基础设施。
 */
import { makeRequest } from './remoteApi';
import type {
  CreateServerRequest,
  CreateServerResponse,
  GetServerResponse,
  ListServersResponse,
  RegenerateTokenResponse,
  UpdateServerRequest,
  ServerWithExecutors,
} from 'shared/server-types';

/**
 * 获取当前组织的服务器列表
 * @param organizationId - 组织 ID
 * @returns 服务器列表（含执行器信息）和总数
 */
export async function listServers(
  organizationId: string
): Promise<ListServersResponse> {
  const response = await makeRequest(
    `/v1/organizations/${organizationId}/servers`
  );
  if (!response.ok) {
    const error = await response.json();
    console.error('[serverApi] listServers failed:', error);
    throw new Error(error.message || 'Failed to list servers');
  }
  return response.json();
}

/**
 * 创建新服务器
 * @param organizationId - 组织 ID
 * @param data - 创建服务器请求数据
 * @returns 新创建的服务器、Agent Token 和 Docker 启动命令
 */
export async function createServer(
  organizationId: string,
  data: CreateServerRequest
): Promise<CreateServerResponse> {
  const response = await makeRequest(
    `/v1/organizations/${organizationId}/servers`,
    {
      method: 'POST',
      body: JSON.stringify(data),
    }
  );
  if (!response.ok) {
    const error = await response.json();
    console.error('[serverApi] createServer failed:', error);
    throw new Error(error.message || 'Failed to create server');
  }
  return response.json();
}

/**
 * 获取服务器详情
 * @param organizationId - 组织 ID
 * @param serverId - 服务器 ID
 * @returns 服务器详情（含执行器信息）
 */
export async function getServer(
  organizationId: string,
  serverId: string
): Promise<GetServerResponse> {
  const response = await makeRequest(
    `/v1/organizations/${organizationId}/servers/${serverId}`
  );
  if (!response.ok) {
    const error = await response.json();
    console.error('[serverApi] getServer failed:', error);
    throw new Error(error.message || 'Failed to get server details');
  }
  return response.json();
}

/**
 * 更新服务器信息
 * @param organizationId - 组织 ID
 * @param serverId - 服务器 ID
 * @param data - 更新数据（部分字段）
 * @returns 更新后的服务器信息
 */
export async function updateServer(
  organizationId: string,
  serverId: string,
  data: UpdateServerRequest
): Promise<ServerWithExecutors> {
  const response = await makeRequest(
    `/v1/organizations/${organizationId}/servers/${serverId}`,
    {
      method: 'PUT',
      body: JSON.stringify(data),
    }
  );
  if (!response.ok) {
    const error = await response.json();
    console.error('[serverApi] updateServer failed:', error);
    throw new Error(error.message || 'Failed to update server');
  }
  return response.json();
}

/**
 * 删除服务器
 * @param organizationId - 组织 ID
 * @param serverId - 服务器 ID
 */
export async function deleteServer(
  organizationId: string,
  serverId: string
): Promise<void> {
  const response = await makeRequest(
    `/v1/organizations/${organizationId}/servers/${serverId}`,
    {
      method: 'DELETE',
    }
  );
  if (!response.ok) {
    const error = await response.json();
    console.error('[serverApi] deleteServer failed:', error);
    throw new Error(error.message || 'Failed to delete server');
  }
}

/**
 * 重新生成服务器的 Agent Token
 * @param organizationId - 组织 ID
 * @param serverId - 服务器 ID
 * @returns 新的 Agent Token 和 Docker 启动命令
 */
export async function regenerateAgentToken(
  organizationId: string,
  serverId: string
): Promise<RegenerateTokenResponse> {
  const response = await makeRequest(
    `/v1/organizations/${organizationId}/servers/${serverId}/regenerate-token`,
    {
      method: 'POST',
    }
  );
  if (!response.ok) {
    const error = await response.json();
    console.error('[serverApi] regenerateAgentToken failed:', error);
    throw new Error(error.message || 'Failed to regenerate agent token');
  }
  return response.json();
}
