/**
 * 获取服务器列表的 React Query Hook
 *
 * 支持按组织 ID 筛选服务器列表，自动处理认证状态。
 */
import { useQuery } from '@tanstack/react-query';
import { listServers } from '@/lib/serverApi';
import { useAuth } from '@/hooks/auth/useAuth';
import { serverKeys } from './serverKeys';
import type { ListServersResponse } from 'shared/server-types';

/**
 * 获取当前组织的服务器列表
 * @param organizationId - 组织 ID，为空时禁用查询
 */
export function useServers(organizationId: string | null) {
  const { isSignedIn } = useAuth();
  const enabled = isSignedIn && !!organizationId;

  return useQuery<ListServersResponse>({
    queryKey: serverKeys.list(organizationId || ''),
    queryFn: () => listServers(organizationId!),
    enabled,
    staleTime: 30 * 1000, // 30秒，服务器状态变化较频繁
    refetchInterval: 30 * 1000, // 每30秒自动刷新服务器状态
  });
}
