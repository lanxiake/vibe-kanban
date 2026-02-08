/**
 * Server 管理的 Mutation Hooks
 *
 * 提供创建、更新、删除服务器和重新生成 Token 的 mutation hooks。
 */
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  createServer,
  updateServer,
  deleteServer,
  regenerateAgentToken,
} from '@/lib/serverApi';
import type {
  CreateServerRequest,
  CreateServerResponse,
  UpdateServerRequest,
  ServerWithExecutors,
  RegenerateTokenResponse,
} from 'shared/server-types';
import { serverKeys } from './serverKeys';

interface UseServerMutationsOptions {
  onCreateSuccess?: (result: CreateServerResponse) => void;
  onCreateError?: (err: unknown) => void;
  onUpdateSuccess?: (result: ServerWithExecutors) => void;
  onUpdateError?: (err: unknown) => void;
  onDeleteSuccess?: () => void;
  onDeleteError?: (err: unknown) => void;
  onRegenerateTokenSuccess?: (result: RegenerateTokenResponse) => void;
  onRegenerateTokenError?: (err: unknown) => void;
}

/**
 * 服务器管理的 mutation hooks 集合
 * @param organizationId - 当前组织 ID
 * @param options - 各 mutation 的成功/失败回调
 */
export function useServerMutations(
  organizationId: string,
  options?: UseServerMutationsOptions
) {
  const queryClient = useQueryClient();

  /** 创建新服务器 */
  const create = useMutation<
    CreateServerResponse,
    unknown,
    CreateServerRequest
  >({
    mutationKey: ['createServer', organizationId],
    mutationFn: (data) => createServer(organizationId, data),
    onSuccess: (result) => {
      console.log('[useServerMutations] Server created:', result.server.id);
      queryClient.invalidateQueries({
        queryKey: serverKeys.list(organizationId),
      });
      options?.onCreateSuccess?.(result);
    },
    onError: (err) => {
      console.error('[useServerMutations] Failed to create server:', err);
      options?.onCreateError?.(err);
    },
  });

  /** 更新服务器信息 */
  const update = useMutation<
    ServerWithExecutors,
    unknown,
    { serverId: string; data: UpdateServerRequest }
  >({
    mutationKey: ['updateServer', organizationId],
    mutationFn: ({ serverId, data }) =>
      updateServer(organizationId, serverId, data),
    onSuccess: (result, variables) => {
      console.log(
        '[useServerMutations] Server updated:',
        variables.serverId
      );
      queryClient.invalidateQueries({
        queryKey: serverKeys.list(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: serverKeys.detail(organizationId, variables.serverId),
      });
      options?.onUpdateSuccess?.(result);
    },
    onError: (err) => {
      console.error('[useServerMutations] Failed to update server:', err);
      options?.onUpdateError?.(err);
    },
  });

  /** 删除服务器 */
  const remove = useMutation<void, unknown, string>({
    mutationKey: ['deleteServer', organizationId],
    mutationFn: (serverId) => deleteServer(organizationId, serverId),
    onSuccess: (_data, serverId) => {
      console.log('[useServerMutations] Server deleted:', serverId);
      queryClient.invalidateQueries({
        queryKey: serverKeys.list(organizationId),
      });
      options?.onDeleteSuccess?.();
    },
    onError: (err) => {
      console.error('[useServerMutations] Failed to delete server:', err);
      options?.onDeleteError?.(err);
    },
  });

  /** 重新生成 Agent Token */
  const regenerateToken = useMutation<
    RegenerateTokenResponse,
    unknown,
    string
  >({
    mutationKey: ['regenerateAgentToken', organizationId],
    mutationFn: (serverId) =>
      regenerateAgentToken(organizationId, serverId),
    onSuccess: (result) => {
      console.log('[useServerMutations] Agent token regenerated');
      options?.onRegenerateTokenSuccess?.(result);
    },
    onError: (err) => {
      console.error(
        '[useServerMutations] Failed to regenerate token:',
        err
      );
      options?.onRegenerateTokenError?.(err);
    },
  });

  return { create, update, remove, regenerateToken };
}
