/**
 * AddServerDialog - 添加服务器对话框
 *
 * 使用 NiceModal 实现的添加服务器对话框，创建成功后显示 Agent Token 和 Docker 启动命令。
 */
import { useState, useEffect } from 'react';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  CheckCircleIcon,
  CopyIcon,
  WarningIcon,
} from '@phosphor-icons/react';
import { defineModal } from '@/lib/modals';
import { useServerMutations } from '@/hooks/useServerMutations';
import { useOrganizationStore } from '@/stores/useOrganizationStore';
import type { CreateServerResponse } from 'shared/server-types';

export interface AddServerDialogProps {}

export type AddServerDialogResult =
  | { status: 'created'; response: CreateServerResponse }
  | { status: 'canceled' };

const AddServerDialogImpl = NiceModal.create<AddServerDialogProps>(() => {
  const modal = useModal();
  const selectedOrgId = useOrganizationStore((s) => s.selectedOrgId);

  // 表单状态
  const [name, setName] = useState('');
  const [host, setHost] = useState('');
  const [port, setPort] = useState('9999');
  const [description, setDescription] = useState('');
  const [tags, setTags] = useState('');
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

  // 创建成功后的响应数据
  const [createdResponse, setCreatedResponse] =
    useState<CreateServerResponse | null>(null);

  // 复制状态
  const [copiedToken, setCopiedToken] = useState(false);
  const [copiedCommand, setCopiedCommand] = useState(false);

  const { create } = useServerMutations(selectedOrgId || '', {
    onCreateSuccess: (response) => {
      setCreatedResponse(response);
    },
    onCreateError: () => {
      // 错误由 create.isError / create.error 状态驱动 UI 展示
    },
  });

  // 重置表单
  useEffect(() => {
    if (modal.visible) {
      setName('');
      setHost('');
      setPort('9999');
      setDescription('');
      setTags('');
      setCreatedResponse(null);
      setCopiedToken(false);
      setCopiedCommand(false);
      setValidationErrors({});
    }
  }, [modal.visible]);

  /** 验证表单输入 */
  const validateForm = (): boolean => {
    const errors: Record<string, string> = {};

    if (!name.trim()) {
      errors.name = '请输入服务器名称';
    } else if (name.trim().length > 255) {
      errors.name = '服务器名称不能超过 255 个字符';
    }

    if (!host.trim()) {
      errors.host = '请输入主机地址';
    } else {
      // 验证主机格式：IP 地址或域名
      const hostTrimmed = host.trim();
      const ipv4Pattern = /^(\d{1,3}\.){3}\d{1,3}$/;
      const hostnamePattern = /^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$/;
      if (!ipv4Pattern.test(hostTrimmed) && !hostnamePattern.test(hostTrimmed)) {
        errors.host = '请输入有效的 IP 地址或域名';
      }
    }

    const portNum = parseInt(port);
    if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
      errors.port = '端口范围：1-65535';
    }

    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  /** 创建服务器 */
  const handleCreate = () => {
    if (!validateForm()) return;

    const tagArray = tags
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);

    create.mutate({
      name: name.trim(),
      host: host.trim(),
      port: parseInt(port),
      description: description.trim() || null,
      tags: tagArray.length > 0 ? tagArray : null,
      ssh_config: null, // Phase 1 暂不支持 SSH
    });
  };

  /** 复制到剪贴板 */
  const handleCopy = async (text: string, type: 'token' | 'command') => {
    try {
      await navigator.clipboard.writeText(text);
      if (type === 'token') {
        setCopiedToken(true);
        setTimeout(() => setCopiedToken(false), 2000);
      } else {
        setCopiedCommand(true);
        setTimeout(() => setCopiedCommand(false), 2000);
      }
    } catch {
      // 剪贴板 API 不可用时静默失败
    }
  };

  /** 完成并关闭 */
  const handleDone = () => {
    if (createdResponse) {
      modal.resolve({
        status: 'created',
        response: createdResponse,
      } as AddServerDialogResult);
    } else {
      modal.resolve({ status: 'canceled' } as AddServerDialogResult);
    }
    modal.hide();
  };

  const handleCancel = () => {
    modal.resolve({ status: 'canceled' } as AddServerDialogResult);
    modal.hide();
  };

  const handleOpenChange = (open: boolean) => {
    if (!open && !create.isPending) {
      handleCancel();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (
      e.key === 'Enter' &&
      !createdResponse &&
      !create.isPending
    ) {
      e.preventDefault();
      handleCreate();
    }
  };

  return (
    <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {createdResponse ? '服务器创建成功' : '添加服务器'}
          </DialogTitle>
          <DialogDescription>
            {createdResponse
              ? '请保存 Agent Token 并在服务器上运行 Docker 命令'
              : '配置新的 AI 工具执行服务器'}
          </DialogDescription>
        </DialogHeader>

        {!createdResponse ? (
          // 创建表单
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="server-name">
                服务器名称 <span className="text-error">*</span>
              </Label>
              <Input
                id="server-name"
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                  setValidationErrors((prev) => ({ ...prev, name: '' }));
                }}
                onKeyDown={handleKeyDown}
                placeholder="例如：开发服务器"
                autoFocus
                disabled={create.isPending}
              />
              {validationErrors.name && (
                <p className="text-xs text-error">{validationErrors.name}</p>
              )}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="server-host">
                  主机地址 <span className="text-error">*</span>
                </Label>
                <Input
                  id="server-host"
                  value={host}
                  onChange={(e) => {
                    setHost(e.target.value);
                    setValidationErrors((prev) => ({ ...prev, host: '' }));
                  }}
                  onKeyDown={handleKeyDown}
                  placeholder="例如：192.168.1.100"
                  disabled={create.isPending}
                />
                {validationErrors.host && (
                  <p className="text-xs text-error">{validationErrors.host}</p>
                )}
              </div>

              <div className="space-y-2">
                <Label htmlFor="server-port">端口</Label>
                <Input
                  id="server-port"
                  type="number"
                  min={1}
                  max={65535}
                  value={port}
                  onChange={(e) => {
                    setPort(e.target.value);
                    setValidationErrors((prev) => ({ ...prev, port: '' }));
                  }}
                  onKeyDown={handleKeyDown}
                  placeholder="9999"
                  disabled={create.isPending}
                />
                {validationErrors.port && (
                  <p className="text-xs text-error">{validationErrors.port}</p>
                )}
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="server-description">描述</Label>
              <Textarea
                id="server-description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="服务器用途说明（可选）"
                rows={2}
                disabled={create.isPending}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="server-tags">标签</Label>
              <Input
                id="server-tags"
                value={tags}
                onChange={(e) => setTags(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="用逗号分隔，例如：生产,高性能"
                disabled={create.isPending}
              />
            </div>

            {create.isError && (
              <Alert variant="destructive">
                <WarningIcon className="h-4 w-4" />
                <AlertDescription>
                  {create.error instanceof Error
                    ? create.error.message
                    : '创建服务器失败，请重试'}
                </AlertDescription>
              </Alert>
            )}
          </div>
        ) : (
          // 创建成功 - 显示 Token 和 Docker 命令
          <div className="space-y-4">
            <Alert>
              <CheckCircleIcon className="h-4 w-4 text-success" />
              <AlertDescription>
                服务器 <strong>{createdResponse.server.name}</strong>{' '}
                已创建，请按以下步骤部署 Agent
              </AlertDescription>
            </Alert>

            {/* Agent Token */}
            <div className="space-y-2">
              <Label>Agent Token（仅显示一次，请妥善保存）</Label>
              <div className="flex gap-2">
                <Input
                  value={createdResponse.agent_token}
                  readOnly
                  className="font-mono text-sm"
                />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    handleCopy(createdResponse.agent_token, 'token')
                  }
                >
                  {copiedToken ? (
                    <CheckCircleIcon className="size-4 text-success" />
                  ) : (
                    <CopyIcon className="size-4" />
                  )}
                </Button>
              </div>
            </div>

            {/* Docker 启动命令 */}
            <div className="space-y-2">
              <Label>Docker 启动命令</Label>
              <div className="relative">
                <Textarea
                  value={createdResponse.docker_command}
                  readOnly
                  rows={6}
                  className="font-mono text-xs pr-12"
                />
                <Button
                  variant="outline"
                  size="sm"
                  className="absolute top-2 right-2"
                  onClick={() =>
                    handleCopy(createdResponse.docker_command, 'command')
                  }
                >
                  {copiedCommand ? (
                    <CheckCircleIcon className="size-4 text-success" />
                  ) : (
                    <CopyIcon className="size-4" />
                  )}
                </Button>
              </div>
              <p className="text-xs text-low">
                在目标服务器上运行此命令以启动 Agent 守护进程
              </p>
            </div>
          </div>
        )}

        <DialogFooter>
          {!createdResponse ? (
            <>
              <Button
                variant="outline"
                onClick={handleCancel}
                disabled={create.isPending}
              >
                取消
              </Button>
              <Button
                onClick={handleCreate}
                disabled={create.isPending}
              >
                {create.isPending ? '创建中...' : '创建服务器'}
              </Button>
            </>
          ) : (
            <Button onClick={handleDone}>完成</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});

export const AddServerDialog = defineModal<
  AddServerDialogProps,
  AddServerDialogResult
>(AddServerDialogImpl);
