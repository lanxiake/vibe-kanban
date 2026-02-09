/**
 * ServersContainer - 服务器列表容器组件（Container）
 *
 * 管理服务器列表的状态和业务逻辑，包括数据获取、搜索过滤、对话框交互。
 */
import { useState, useMemo, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { ServersView } from '../views/ServersView';
import { useServers } from '@/hooks/useServers';
import { useOrganizationStore } from '@/stores/useOrganizationStore';
import { AddServerDialog } from '@/components/dialogs/servers/AddServerDialog';

export function ServersContainer() {
  const selectedOrgId = useOrganizationStore((s) => s.selectedOrgId);
  const navigate = useNavigate();
  const [searchQuery, setSearchQuery] = useState('');

  // 获取服务器列表
  const { data, isLoading } = useServers(selectedOrgId);

  // 搜索过滤
  const filteredServers = useMemo(() => {
    const servers = data?.servers ?? [];
    if (!searchQuery.trim()) return servers;

    const query = searchQuery.toLowerCase();
    return servers.filter(
      (server) =>
        server.name.toLowerCase().includes(query) ||
        server.host.toLowerCase().includes(query) ||
        server.description?.toLowerCase().includes(query) ||
        server.tags.some((tag) => tag.toLowerCase().includes(query))
    );
  }, [data?.servers, searchQuery]);

  /** 打开添加服务器对话框 */
  const handleAddServer = useCallback(async () => {
    try {
      await AddServerDialog.show({});
    } catch {
      // Dialog cancelled
    }
  }, []);

  /** 点击服务器卡片，导航到详情页 */
  const handleServerClick = useCallback(
    (serverId: string) => {
      navigate(`/servers/${serverId}`);
    },
    [navigate]
  );

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
