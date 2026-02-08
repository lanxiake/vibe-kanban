/**
 * 获取单个服务器详情的 React Query Hook
 */
import { useQuery } from '@tanstack/react-query';
import { getServer } from '@/lib/serverApi';
import { useAuth } from '@/hooks/auth/useAuth';
import { serverKeys } from './serverKeys';
import type { GetServerResponse } from 'shared/server-types';

/**
 * 获取服务器详情（含执行器信息）
 * @param organizationId - 组织 ID
 * @param serverId - 服务器 ID，为空时禁用查询
 */
export function useServer(
  organizationId: string | null,
  serverId: string | null
) {
  const { isSignedIn } = useAuth();
  const enabled = isSignedIn && !!organizationId && !!serverId;

  return useQuery<GetServerResponse>({
    queryKey: serverKeys.detail(organizationId || '', serverId || ''),
    queryFn: () => getServer(organizationId!, serverId!),
    enabled,
    staleTime: 15 * 1000, // 15秒
    refetchInterval: 15 * 1000, // 详情页刷新更频繁
  });
}
