/**
 * Server 相关的 React Query 缓存键定义
 */
export const serverKeys = {
  all: ['servers'] as const,
  list: (orgId: string) => ['servers', orgId, 'list'] as const,
  detail: (orgId: string, serverId: string) =>
    ['servers', orgId, serverId] as const,
};
