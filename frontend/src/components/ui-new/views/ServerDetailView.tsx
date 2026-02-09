/**
 * ServerDetailView - 服务器详情视图组件（View）
 *
 * 纯展示组件，显示服务器的完整信息：
 * - 基本信息（名称、主机、端口、描述）
 * - 连接状态和 Agent 信息
 * - 系统资源使用（CPU、内存、磁盘）
 * - 执行器列表和详细信息
 * - 操作按钮（编辑、删除、重新生成 Token）
 */
import {
  ArrowLeftIcon,
  CheckCircleIcon,
  ClockIcon,
  CopyIcon,
  CpuIcon,
  GearIcon,
  HardDriveIcon,
  HardDrivesIcon,
  MemoryIcon,
  PencilSimpleIcon,
  TrashIcon,
  WarningCircleIcon,
  XCircleIcon,
  type Icon,
} from '@phosphor-icons/react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import type {
  ServerExecutor,
  ServerStatus,
  ServerWithExecutors,
  SystemInfo,
  SystemStats,
} from 'shared/server-types';

interface ServerDetailViewProps {
  server: ServerWithExecutors;
  onBack: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onRegenerateToken: () => void;
  onInstallGuide: () => void;
  isDeleting: boolean;
  isRegenerating: boolean;
}

/**
 * 获取服务器状态显示配置
 */
function getStatusConfig(status: ServerStatus) {
  switch (status) {
    case 'online':
      return {
        icon: CheckCircleIcon,
        color: 'text-success',
        bgColor: 'bg-success/10',
        label: '在线',
      };
    case 'offline':
      return {
        icon: XCircleIcon,
        color: 'text-muted',
        bgColor: 'bg-secondary',
        label: '离线',
      };
    case 'pending':
      return {
        icon: ClockIcon,
        color: 'text-warning',
        bgColor: 'bg-warning/10',
        label: '等待连接',
      };
    case 'error':
      return {
        icon: WarningCircleIcon,
        color: 'text-error',
        bgColor: 'bg-error/10',
        label: '错误',
      };
    default:
      return {
        icon: ClockIcon,
        color: 'text-muted',
        bgColor: 'bg-secondary',
        label: '未知',
      };
  }
}

/**
 * 格式化时间戳为本地时间
 */
function formatTime(timestamp: string | null): string {
  if (!timestamp) return '从未';
  const date = new Date(timestamp);
  return date.toLocaleString('zh-CN');
}

/**
 * 格式化资源百分比
 */
function formatPercent(value: number): string {
  return `${Math.round(value)}%`;
}

/**
 * 格式化存储容量
 */
function formatGB(value: number): string {
  return `${value.toFixed(1)} GB`;
}

/**
 * 渲染资源使用详情条
 */
function DetailResourceBar({
  label,
  icon: IconComponent,
  percent,
  used,
  total,
}: {
  label: string;
  icon: Icon;
  percent: number;
  used?: string;
  total?: string;
}) {
  const colorClass =
    percent > 90
      ? 'bg-error'
      : percent > 75
        ? 'bg-warning'
        : 'bg-success';

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <IconComponent className="size-4 text-low" weight="duotone" />
          <span className="text-sm text-normal">{label}</span>
        </div>
        <div className="text-sm font-medium text-normal">
          {formatPercent(percent)}
          {used && total && (
            <span className="text-low ml-2">
              ({used} / {total})
            </span>
          )}
        </div>
      </div>
      <div className="h-2 bg-secondary rounded-full overflow-hidden">
        <div
          className={cn('h-full transition-all rounded-full', colorClass)}
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>
    </div>
  );
}

/**
 * 信息行组件
 */
function InfoRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string | null | undefined;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-border last:border-0">
      <span className="text-sm text-low">{label}</span>
      <span
        className={cn(
          'text-sm text-normal',
          mono && 'font-mono'
        )}
      >
        {value || '-'}
      </span>
    </div>
  );
}

/**
 * 执行器详情卡片
 */
function ExecutorCard({ executor }: { executor: ServerExecutor }) {
  const statusConfig = {
    available: { color: 'text-success', bg: 'bg-success/10', label: '可用' },
    busy: { color: 'text-warning', bg: 'bg-warning/10', label: '忙碌' },
    error: { color: 'text-error', bg: 'bg-error/10', label: '错误' },
    unknown: { color: 'text-muted', bg: 'bg-secondary', label: '未知' },
  };

  const config = statusConfig[executor.status] || statusConfig.unknown;

  /** 计算执行成功率 */
  const successRate =
    executor.total_executions > 0
      ? Math.round(
          (executor.successful_executions / executor.total_executions) * 100
        )
      : 0;

  return (
    <div className="bg-secondary rounded-sm border border-border p-base">
      {/* 执行器名称和状态 */}
      <div className="flex items-center justify-between mb-base">
        <div className="flex items-center gap-2">
          <GearIcon className="size-4 text-brand" weight="duotone" />
          <span className="text-base font-medium text-normal">
            {executor.executor_name || executor.executor_type}
          </span>
          {executor.version && (
            <span className="text-xs text-low">v{executor.version}</span>
          )}
        </div>
        <span
          className={cn('px-2 py-0.5 rounded text-xs', config.bg, config.color)}
        >
          {config.label}
        </span>
      </div>

      {/* 执行统计 */}
      <div className="grid grid-cols-3 gap-base mb-base">
        <div>
          <div className="text-xs text-low">总执行</div>
          <div className="text-base font-medium text-normal">
            {executor.total_executions}
          </div>
        </div>
        <div>
          <div className="text-xs text-low">成功率</div>
          <div className="text-base font-medium text-normal">
            {executor.total_executions > 0 ? `${successRate}%` : '-'}
          </div>
        </div>
        <div>
          <div className="text-xs text-low">最后使用</div>
          <div className="text-xs text-normal">
            {formatTime(executor.last_used_at)}
          </div>
        </div>
      </div>

      {/* 能力标签 */}
      {executor.capabilities.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {executor.capabilities.map((cap) => (
            <span
              key={cap}
              className="px-2 py-0.5 bg-primary text-xs text-low rounded"
            >
              {cap}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

export function ServerDetailView({
  server,
  onBack,
  onEdit,
  onDelete,
  onRegenerateToken,
  onInstallGuide,
  isDeleting,
  isRegenerating,
}: ServerDetailViewProps) {
  const statusConfig = getStatusConfig(server.status);
  const StatusIcon = statusConfig.icon;
  const stats = server.system_stats as SystemStats | null;
  const sysInfo = server.system_info as SystemInfo | null;

  return (
    <div className="flex flex-col h-full bg-primary">
      {/* Header */}
      <div className="px-base py-base border-b border-border">
        <div className="flex items-center gap-base mb-base">
          <Button
            variant="ghost"
            size="sm"
            onClick={onBack}
            className="shrink-0"
          >
            <ArrowLeftIcon className="size-4" />
          </Button>
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <HardDrivesIcon
              className="size-6 text-brand shrink-0"
              weight="duotone"
            />
            <div className="min-w-0">
              <h1 className="text-xl font-semibold text-normal truncate">
                {server.name}
              </h1>
              <p className="text-sm text-low truncate">
                {server.host}:{server.port}
              </p>
            </div>
          </div>
          <div
            className={cn(
              'flex items-center gap-1.5 px-3 py-1.5 rounded-sm',
              statusConfig.bgColor
            )}
          >
            <StatusIcon
              className={cn('size-4', statusConfig.color)}
              weight="fill"
            />
            <span className={cn('text-sm font-medium', statusConfig.color)}>
              {statusConfig.label}
            </span>
          </div>
        </div>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={onInstallGuide}>
            <CopyIcon className="size-4 mr-1.5" />
            安装指引
          </Button>
          <Button variant="outline" size="sm" onClick={onEdit}>
            <PencilSimpleIcon className="size-4 mr-1.5" />
            编辑
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={onRegenerateToken}
            disabled={isRegenerating}
          >
            {isRegenerating ? '生成中...' : '重新生成 Token'}
          </Button>
          <div className="flex-1" />
          <Button
            variant="destructive"
            size="sm"
            onClick={onDelete}
            disabled={isDeleting}
          >
            <TrashIcon className="size-4 mr-1.5" />
            {isDeleting ? '删除中...' : '删除'}
          </Button>
        </div>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-y-auto p-base">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-base max-w-6xl">
          {/* 左列：基本信息 + Agent 信息 */}
          <div className="space-y-base">
            {/* 基本信息 */}
            <div className="bg-secondary rounded-sm border border-border p-base">
              <h2 className="text-base font-semibold text-normal mb-base">
                基本信息
              </h2>
              <InfoRow label="名称" value={server.name} />
              <InfoRow label="主机" value={server.host} mono />
              <InfoRow label="端口" value={String(server.port)} mono />
              <InfoRow label="描述" value={server.description} />
              <InfoRow label="创建时间" value={formatTime(server.created_at)} />
              <InfoRow label="更新时间" value={formatTime(server.updated_at)} />
              {server.tags.length > 0 && (
                <div className="flex items-center justify-between py-2">
                  <span className="text-sm text-low">标签</span>
                  <div className="flex flex-wrap gap-1.5">
                    {server.tags.map((tag) => (
                      <span
                        key={tag}
                        className="px-2 py-0.5 bg-primary text-xs text-low rounded"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Agent 连接信息 */}
            <div className="bg-secondary rounded-sm border border-border p-base">
              <h2 className="text-base font-semibold text-normal mb-base">
                Agent 信息
              </h2>
              <InfoRow label="Agent ID" value={server.agent_id} mono />
              <InfoRow label="Agent 版本" value={server.agent_version} />
              <InfoRow
                label="最后心跳"
                value={formatTime(server.last_heartbeat_at)}
              />
              {server.connection_error && (
                <div className="mt-base p-2 bg-error/10 rounded text-sm text-error">
                  {server.connection_error}
                </div>
              )}
              {server.status === 'pending' && (
                <div className="mt-base p-2 bg-warning/10 rounded text-sm text-warning">
                  Agent 尚未连接，请在服务器上部署 Agent Daemon
                </div>
              )}
            </div>

            {/* 系统信息 */}
            {sysInfo && (
              <div className="bg-secondary rounded-sm border border-border p-base">
                <h2 className="text-base font-semibold text-normal mb-base">
                  系统信息
                </h2>
                <InfoRow label="操作系统" value={sysInfo.os} />
                <InfoRow label="架构" value={sysInfo.arch} />
                <InfoRow label="主机名" value={sysInfo.hostname} />
                <InfoRow label="CPU 核心" value={String(sysInfo.cpu_cores)} />
                <InfoRow
                  label="总内存"
                  value={formatGB(sysInfo.total_memory_gb)}
                />
                <InfoRow
                  label="总磁盘"
                  value={formatGB(sysInfo.disk_total_gb)}
                />
              </div>
            )}
          </div>

          {/* 右列：资源使用 + 执行器 */}
          <div className="space-y-base">
            {/* 资源使用 */}
            {stats ? (
              <div className="bg-secondary rounded-sm border border-border p-base">
                <h2 className="text-base font-semibold text-normal mb-base">
                  资源使用
                </h2>
                <div className="space-y-4">
                  <DetailResourceBar
                    label="CPU"
                    icon={CpuIcon}
                    percent={stats.cpu_usage_percent}
                  />
                  <DetailResourceBar
                    label="内存"
                    icon={MemoryIcon}
                    percent={stats.memory_usage_percent}
                    used={formatGB(
                      (sysInfo?.total_memory_gb ?? 0) -
                        stats.memory_available_gb
                    )}
                    total={
                      sysInfo ? formatGB(sysInfo.total_memory_gb) : undefined
                    }
                  />
                  <DetailResourceBar
                    label="磁盘"
                    icon={HardDriveIcon}
                    percent={stats.disk_usage_percent}
                    used={formatGB(
                      (sysInfo?.disk_total_gb ?? 0) - stats.disk_available_gb
                    )}
                    total={
                      sysInfo ? formatGB(sysInfo.disk_total_gb) : undefined
                    }
                  />
                </div>
                <div className="mt-base pt-base border-t border-border">
                  <InfoRow
                    label="正在运行的任务"
                    value={String(stats.running_tasks)}
                  />
                </div>
              </div>
            ) : (
              <div className="bg-secondary rounded-sm border border-border p-base">
                <h2 className="text-base font-semibold text-normal mb-base">
                  资源使用
                </h2>
                <p className="text-sm text-low text-center py-4">
                  等待 Agent 上报资源数据...
                </p>
              </div>
            )}

            {/* 执行器列表 */}
            <div className="bg-secondary rounded-sm border border-border p-base">
              <div className="flex items-center justify-between mb-base">
                <h2 className="text-base font-semibold text-normal">
                  执行器
                </h2>
                <span className="text-sm text-low">
                  {server.executors.length} 个
                </span>
              </div>
              {server.executors.length > 0 ? (
                <div className="space-y-base">
                  {server.executors.map((executor) => (
                    <ExecutorCard key={executor.id} executor={executor} />
                  ))}
                </div>
              ) : (
                <p className="text-sm text-low text-center py-4">
                  暂无执行器，Agent 连接后将自动发现
                </p>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
