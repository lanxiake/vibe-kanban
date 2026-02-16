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
 * 安全地解析错误响应，兼容非 JSON 响应（如 502 HTML 页面）
 * @param response - fetch 响应对象
 * @param fallbackMessage - JSON 解析失败时的兜底错误信息
 */
async function handleErrorResponse(
  response: Response,
  fallbackMessage: string
): Promise<never> {
  let message = fallbackMessage;
  try {
    const contentType = response.headers.get('content-type') || '';
    if (contentType.includes('application/json')) {
      const error = await response.json();
      message = error.message || fallbackMessage;
    } else {
      const text = await response.text();
      message = text.slice(0, 200) || `${fallbackMessage} (HTTP ${response.status})`;
    }
  } catch {
    message = `${fallbackMessage} (HTTP ${response.status})`;
  }
  throw new Error(message);
}

/**
 * 安全地解析成功响应的 JSON 数据
 * @param response - fetch 响应对象
 */
async function parseJsonResponse<T>(response: Response): Promise<T> {
  return response.json() as Promise<T>;
}

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
    await handleErrorResponse(response, 'Failed to list servers');
  }
  return parseJsonResponse<ListServersResponse>(response);
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
    await handleErrorResponse(response, 'Failed to create server');
  }
  return parseJsonResponse<CreateServerResponse>(response);
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
    await handleErrorResponse(response, 'Failed to get server details');
  }
  return parseJsonResponse<GetServerResponse>(response);
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
    await handleErrorResponse(response, 'Failed to update server');
  }
  return parseJsonResponse<ServerWithExecutors>(response);
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
    await handleErrorResponse(response, 'Failed to delete server');
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
    await handleErrorResponse(response, 'Failed to regenerate agent token');
  }
  return parseJsonResponse<RegenerateTokenResponse>(response);
}
