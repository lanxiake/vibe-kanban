/**
 * ServerCard - 服务器卡片组件（Primitive）
 *
 * 显示单个服务器的状态、资源使用情况和执行器信息。
 * 无状态组件，所有数据通过 props 传入。
 */
import {
  CheckCircleIcon,
  ClockIcon,
  CpuIcon,
  HardDriveIcon,
  HardDrivesIcon,
  MemoryIcon,
  WarningCircleIcon,
  XCircleIcon,
  type Icon,
} from '@phosphor-icons/react';
import { cn } from '@/lib/utils';
import type {
  ServerStatus,
  ServerWithExecutors,
  SystemStats,
} from 'shared/server-types';

interface ServerCardProps {
  server: ServerWithExecutors;
  onClick?: () => void;
  onDelete?: () => void;
}

/**
 * 获取服务器状态图标和颜色
 */
function getStatusDisplay(status: ServerStatus) {
  switch (status) {
    case 'online':
      return {
        icon: CheckCircleIcon,
        color: 'text-success',
        label: '在线',
      };
    case 'offline':
      return {
        icon: XCircleIcon,
        color: 'text-muted',
        label: '离线',
      };
    case 'pending':
      return {
        icon: ClockIcon,
        color: 'text-warning',
        label: '等待连接',
      };
    case 'error':
      return {
        icon: WarningCircleIcon,
        color: 'text-error',
        label: '错误',
      };
    default:
      return {
        icon: ClockIcon,
        color: 'text-muted',
        label: '未知',
      };
  }
}

/**
 * 格式化资源使用百分比
 */
function formatPercent(value: number): string {
  return `${Math.round(value)}%`;
}

/**
 * 格式化存储容量（GB）
 */
function formatGB(value: number): string {
  return `${value.toFixed(1)} GB`;
}

/**
 * 渲染资源使用进度条
 */
function ResourceBar({
  label,
  icon: Icon,
  percent,
  available,
}: {
  label: string;
  icon: Icon;
  percent: number;
  available?: string;
}) {
  const colorClass =
    percent > 90
      ? 'bg-error'
      : percent > 75
        ? 'bg-warning'
        : 'bg-success';

  return (
    <div className="flex items-center gap-2">
      <Icon className="size-4 text-low shrink-0" weight="duotone" />
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between text-xs mb-1">
          <span className="text-low">{label}</span>
          <span className="text-normal font-medium">
            {formatPercent(percent)}
            {available && (
              <span className="text-low ml-1">({available})</span>
            )}
          </span>
        </div>
        <div className="h-1.5 bg-secondary rounded-full overflow-hidden">
          <div
            className={cn('h-full transition-all', colorClass)}
            style={{ width: `${Math.min(percent, 100)}%` }}
          />
        </div>
      </div>
    </div>
  );
}

export function ServerCard({ server, onClick }: ServerCardProps) {
  const statusDisplay = getStatusDisplay(server.status);
  const StatusIcon = statusDisplay.icon;
  const stats = server.system_stats as SystemStats | null;

  return (
    <div
      className={cn(
        'bg-primary rounded-sm border border-border p-base',
        'hover:border-brand transition-colors',
        onClick && 'cursor-pointer'
      )}
      onClick={onClick}
    >
      {/* Header: 名称 + 状态 */}
      <div className="flex items-start justify-between mb-base">
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <HardDrivesIcon className="size-5 text-brand shrink-0" weight="duotone" />
          <div className="min-w-0 flex-1">
            <h3 className="text-base font-medium text-normal truncate">
              {server.name}
            </h3>
            <p className="text-sm text-low truncate">{server.host}</p>
          </div>
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          <StatusIcon
            className={cn('size-4', statusDisplay.color)}
            weight="fill"
          />
          <span className={cn('text-xs', statusDisplay.color)}>
            {statusDisplay.label}
          </span>
        </div>
      </div>

      {/* 描述 */}
      {server.description && (
        <p className="text-sm text-low mb-base line-clamp-2">
          {server.description}
        </p>
      )}

      {/* 系统资源统计 */}
      {stats && (
        <div className="space-y-2 mb-base">
          <ResourceBar
            label="CPU"
            icon={CpuIcon}
            percent={stats.cpu_usage_percent}
          />
          <ResourceBar
            label="内存"
            icon={MemoryIcon}
            percent={stats.memory_usage_percent}
            available={formatGB(stats.memory_available_gb)}
          />
          <ResourceBar
            label="磁盘"
            icon={HardDriveIcon}
            percent={stats.disk_usage_percent}
            available={formatGB(stats.disk_available_gb)}
          />
        </div>
      )}

      {/* 执行器列表 */}
      {server.executors.length > 0 && (
        <div className="pt-base border-t border-border">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs text-low">执行器</span>
            <span className="text-xs text-normal font-medium">
              {server.executors.length} 个
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {server.executors.map((executor) => (
              <div
                key={executor.id}
                className={cn(
                  'px-2 py-1 rounded text-xs',
                  executor.status === 'available'
                    ? 'bg-success/10 text-success'
                    : executor.status === 'busy'
                      ? 'bg-warning/10 text-warning'
                      : 'bg-secondary text-low'
                )}
              >
                {executor.executor_type}
                {executor.version && (
                  <span className="ml-1 opacity-70">v{executor.version}</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Tags */}
      {server.tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mt-base">
          {server.tags.map((tag) => (
            <span
              key={tag}
              className="px-2 py-0.5 bg-secondary text-xs text-low rounded"
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
