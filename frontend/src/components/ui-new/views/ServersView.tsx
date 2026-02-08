/**
 * ServersView - 服务器列表视图组件（View）
 *
 * 纯展示组件，显示服务器列表、搜索框、过滤器和统计信息。
 */
import { MagnifyingGlassIcon, PlusIcon } from '@phosphor-icons/react';
import { ServerCard } from '../primitives/ServerCard';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { ServerWithExecutors } from 'shared/server-types';

interface ServersViewProps {
  servers: ServerWithExecutors[];
  searchQuery: string;
  onSearchChange: (query: string) => void;
  onServerClick: (serverId: string) => void;
  onAddServer: () => void;
  isLoading: boolean;
}

/**
 * 计算服务器状态统计
 */
function getServerStats(servers: ServerWithExecutors[]) {
  const online = servers.filter((s) => s.status === 'online').length;
  const offline = servers.filter((s) => s.status === 'offline').length;
  const pending = servers.filter((s) => s.status === 'pending').length;
  const error = servers.filter((s) => s.status === 'error').length;

  return { online, offline, pending, error, total: servers.length };
}

export function ServersView({
  servers,
  searchQuery,
  onSearchChange,
  onServerClick,
  onAddServer,
  isLoading,
}: ServersViewProps) {
  const stats = getServerStats(servers);

  return (
    <div className="flex flex-col h-full bg-primary">
      {/* Header */}
      <div className="px-base py-base border-b border-border">
        <div className="flex items-center justify-between mb-base">
          <div>
            <h1 className="text-xl font-semibold text-normal">服务器管理</h1>
            <p className="text-sm text-low mt-1">
              管理 AI 工具执行服务器和 Agent 守护进程
            </p>
          </div>
          <Button onClick={onAddServer} size="sm">
            <PlusIcon className="size-4 mr-1.5" weight="bold" />
            添加服务器
          </Button>
        </div>

        {/* 统计卡片 */}
        <div className="grid grid-cols-5 gap-base">
          <div className="bg-secondary rounded-sm p-base">
            <div className="text-xs text-low mb-1">总计</div>
            <div className="text-2xl font-semibold text-normal">
              {stats.total}
            </div>
          </div>
          <div className="bg-secondary rounded-sm p-base">
            <div className="text-xs text-low mb-1">在线</div>
            <div className="text-2xl font-semibold text-success">
              {stats.online}
            </div>
          </div>
          <div className="bg-secondary rounded-sm p-base">
            <div className="text-xs text-low mb-1">离线</div>
            <div className="text-2xl font-semibold text-muted">
              {stats.offline}
            </div>
          </div>
          <div className="bg-secondary rounded-sm p-base">
            <div className="text-xs text-low mb-1">等待</div>
            <div className="text-2xl font-semibold text-warning">
              {stats.pending}
            </div>
          </div>
          <div className="bg-secondary rounded-sm p-base">
            <div className="text-xs text-low mb-1">错误</div>
            <div className="text-2xl font-semibold text-error">
              {stats.error}
            </div>
          </div>
        </div>

        {/* 搜索框 */}
        <div className="relative mt-base">
          <MagnifyingGlassIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-low" />
          <Input
            type="text"
            placeholder="搜索服务器名称、主机地址或标签..."
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            className="pl-10"
          />
        </div>
      </div>

      {/* 服务器列表 */}
      <div className="flex-1 overflow-y-auto p-base">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="text-low">加载中...</div>
          </div>
        ) : servers.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <div className="text-low mb-2">
              {searchQuery ? '未找到匹配的服务器' : '还没有服务器'}
            </div>
            {!searchQuery && (
              <Button onClick={onAddServer} variant="outline" size="sm">
                <PlusIcon className="size-4 mr-1.5" weight="bold" />
                添加第一个服务器
              </Button>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-base">
            {servers.map((server) => (
              <ServerCard
                key={server.id}
                server={server}
                onClick={() => onServerClick(server.id)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
