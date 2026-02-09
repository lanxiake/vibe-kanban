/**
 * ServerDetailContainer - 服务器详情容器组件（Container）
 *
 * 管理服务器详情页面的状态和业务逻辑，包括：
 * - 通过 URL 参数获取服务器详情
 * - 处理编辑、删除、Token 重新生成操作
 * - 导航回列表页
 */
import { useCallback, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { ServerDetailView } from '../views/ServerDetailView';
import { useServer } from '@/hooks/useServer';
import { useServerMutations } from '@/hooks/useServerMutations';
import { useOrganizationStore } from '@/stores/useOrganizationStore';
import { AgentInstallGuide } from '../views/AgentInstallGuide';

/**
 * 服务器详情容器：从 URL 参数获取 serverId，管理详情页的所有交互
 */
export function ServerDetailContainer() {
  const { serverId } = useParams<{ serverId: string }>();
  const navigate = useNavigate();
  const selectedOrgId = useOrganizationStore((s) => s.selectedOrgId);

  // 安装指引对话框状态
  const [showInstallGuide, setShowInstallGuide] = useState(false);

  // 获取服务器详情
  const { data, isLoading, error } = useServer(
    selectedOrgId,
    serverId || null
  );

  // 服务器操作 mutations
  const { remove, regenerateToken } = useServerMutations(
    selectedOrgId || '',
    {
      onDeleteSuccess: () => {
        navigate('/servers');
      },
      onRegenerateTokenSuccess: (result) => {
        setShowInstallGuide(true);
        setRegeneratedToken(result.agent_token);
        setRegeneratedCommand(result.docker_command);
      },
    }
  );

  // 重新生成 Token 后的数据
  const [regeneratedToken, setRegeneratedToken] = useState<string | null>(null);
  const [regeneratedCommand, setRegeneratedCommand] = useState<string | null>(
    null
  );

  /** 返回列表页 */
  const handleBack = useCallback(() => {
    navigate('/servers');
  }, [navigate]);

  /** 编辑服务器（TODO: 实现编辑对话框） */
  const handleEdit = useCallback(() => {
    // TODO: 打开编辑对话框
  }, []);

  /** 删除服务器 */
  const handleDelete = useCallback(() => {
    if (!serverId) return;
    if (window.confirm('确定要删除这个服务器吗？此操作不可撤销。')) {
      remove.mutate(serverId);
    }
  }, [serverId, remove]);

  /** 重新生成 Token */
  const handleRegenerateToken = useCallback(() => {
    if (!serverId) return;
    if (window.confirm('重新生成 Token 后，旧 Token 将立即失效。确定继续吗？')) {
      regenerateToken.mutate(serverId);
    }
  }, [serverId, regenerateToken]);

  /** 显示安装指引 */
  const handleInstallGuide = useCallback(() => {
    setShowInstallGuide(true);
  }, []);

  // 加载状态
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full bg-primary">
        <div className="text-low">加载服务器详情...</div>
      </div>
    );
  }

  // 错误状态
  if (error || !data?.server) {
    return (
      <div className="flex flex-col items-center justify-center h-full bg-primary gap-4">
        <div className="text-error">
          {error instanceof Error
            ? error.message
            : '无法加载服务器详情'}
        </div>
        <button
          className="text-sm text-brand hover:underline"
          onClick={handleBack}
        >
          返回服务器列表
        </button>
      </div>
    );
  }

  return (
    <>
      <ServerDetailView
        server={data.server}
        onBack={handleBack}
        onEdit={handleEdit}
        onDelete={handleDelete}
        onRegenerateToken={handleRegenerateToken}
        onInstallGuide={handleInstallGuide}
        isDeleting={remove.isPending}
        isRegenerating={regenerateToken.isPending}
      />
      {showInstallGuide && (
        <AgentInstallGuide
          serverName={data.server.name}
          serverHost={data.server.host}
          serverPort={data.server.port}
          agentToken={regeneratedToken}
          dockerCommand={regeneratedCommand}
          onClose={() => {
            setShowInstallGuide(false);
            setRegeneratedToken(null);
            setRegeneratedCommand(null);
          }}
        />
      )}
    </>
  );
}
