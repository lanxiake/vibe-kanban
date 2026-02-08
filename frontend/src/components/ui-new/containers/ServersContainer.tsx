/**
 * ServersContainer - 服务器列表容器组件（Container）
 *
 * 管理服务器列表的状态和业务逻辑，包括数据获取、搜索过滤、对话框交互。
 */
import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { ServersView } from '../views/ServersView';
import { useServers } from '@/hooks/useServers';
import { useOrganizationStore } from '@/stores/useOrganizationStore';
import { AddServerDialog } from '@/components/dialogs/servers/AddServerDialog';

export function ServersContainer() {
  const navigate = useNavigate();
  const selectedOrgId = useOrganizationStore((s) => s.selectedOrgId);
  const [searchQuery, setSearchQuery] = useState('');

  // 获取服务器列表
  const { data, isLoading } = useServers(selectedOrgId);
  const servers = data?.servers ?? [];

  // 搜索过滤
  const filteredServers = useMemo(() => {
    if (!searchQuery.trim()) return servers;

    const query = searchQuery.toLowerCase();
    return servers.filter(
      (server) =>
        server.name.toLowerCase().includes(query) ||
        server.host.toLowerCase().includes(query) ||
        server.description?.toLowerCase().includes(query) ||
        server.tags.some((tag) => tag.toLowerCase().includes(query))
    );
  }, [servers, searchQuery]);

  /** 打开添加服务器对话框 */
  const handleAddServer = async () => {
    try {
      await AddServerDialog.show();
    } catch {
      // Dialog cancelled
    }
  };

  /** 点击服务器卡片，导航到详情页（TODO: Phase 1 暂不实现详情页） */
  const handleServerClick = (serverId: string) => {
    console.log('[ServersContainer] Server clicked:', serverId);
    // TODO: navigate(`/servers/${serverId}`);
  };

  return (
    <ServersView
      servers={filteredServers}
      searchQuery={searchQuery}
      onSearchChange={setSearchQuery}
      onServerClick={handleServerClick}
      onAddServer={handleAddServer}
      isLoading={isLoading}
    />
  );
}
