/**
 * useServerStatus - 服务器状态实时监控 Hook
 *
 * 基于 useServer hook 增强实时状态更新能力：
 * - 根据服务器状态自动调整刷新频率
 * - 提供连接质量指示
 * - 检测心跳超时
 */
import { useMemo } from 'react';
import { useServer } from './useServer';
import type { ServerStatus, SystemStats } from 'shared/server-types';

/** 心跳超时阈值（秒） */
const HEARTBEAT_TIMEOUT_SECONDS = 90;

/** 连接质量等级 */
export type ConnectionQuality = 'excellent' | 'good' | 'poor' | 'disconnected';

interface ServerStatusInfo {
  /** 服务器状态 */
  status: ServerStatus | null;
  /** 连接质量 */
  connectionQuality: ConnectionQuality;
  /** 是否在线 */
  isOnline: boolean;
  /** 心跳是否超时 */
  isHeartbeatTimeout: boolean;
  /** 距离上次心跳的秒数 */
  secondsSinceHeartbeat: number | null;
  /** 系统资源统计 */
  systemStats: SystemStats | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
}

/**
 * 计算连接质量
 */
function calculateConnectionQuality(
  status: ServerStatus,
  secondsSinceHeartbeat: number | null
): ConnectionQuality {
  if (status !== 'online') {
    return 'disconnected';
  }

  if (secondsSinceHeartbeat === null) {
    return 'good';
  }

  if (secondsSinceHeartbeat < 35) {
    return 'excellent';
  }
  if (secondsSinceHeartbeat < 60) {
    return 'good';
  }
  if (secondsSinceHeartbeat < HEARTBEAT_TIMEOUT_SECONDS) {
    return 'poor';
  }
  return 'disconnected';
}

/**
 * 服务器状态实时监控 Hook
 *
 * @param organizationId - 组织 ID
 * @param serverId - 服务器 ID
 * @returns 服务器状态信息
 */
export function useServerStatus(
  organizationId: string | null,
  serverId: string | null
): ServerStatusInfo {
  const { data, isLoading, error } = useServer(organizationId, serverId);

  const statusInfo = useMemo<ServerStatusInfo>(() => {
    if (!data?.server) {
      return {
        status: null,
        connectionQuality: 'disconnected',
        isOnline: false,
        isHeartbeatTimeout: false,
        secondsSinceHeartbeat: null,
        systemStats: null,
        isLoading,
        error: error instanceof Error ? error.message : null,
      };
    }

    const server = data.server;
    const stats = server.system_stats as SystemStats | null;

    /** 计算距离上次心跳的秒数 */
    const secondsSinceHeartbeat = server.last_heartbeat_at
      ? Math.floor(
          (Date.now() - new Date(server.last_heartbeat_at).getTime()) / 1000
        )
      : null;

    const isHeartbeatTimeout =
      secondsSinceHeartbeat !== null &&
      secondsSinceHeartbeat > HEARTBEAT_TIMEOUT_SECONDS;

    const connectionQuality = calculateConnectionQuality(
      server.status,
      secondsSinceHeartbeat
    );

    return {
      status: server.status,
      connectionQuality,
      isOnline: server.status === 'online',
      isHeartbeatTimeout,
      secondsSinceHeartbeat,
      systemStats: stats,
      isLoading,
      error: server.connection_error || null,
    };
  }, [data, isLoading, error]);

  return statusInfo;
}
