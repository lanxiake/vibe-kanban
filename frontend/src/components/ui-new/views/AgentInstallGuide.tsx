/**
 * AgentInstallGuide - Agent 安装指引视图组件（View）
 *
 * 纯展示组件，提供 Agent Daemon 的安装部署指引，包含：
 * - 环境要求说明
 * - Docker 安装命令
 * - 环境变量配置说明
 * - Agent Token 展示（如有）
 * - 常见问题和故障排查
 */
import {
  XIcon,
  TerminalIcon,
  InfoIcon,
  WarningIcon,
} from '@phosphor-icons/react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { CopyableCodeBlock } from '@/components/ui/copyable-code-block';

interface AgentInstallGuideProps {
  serverName: string;
  serverHost: string;
  serverPort: number;
  agentToken?: string | null;
  dockerCommand?: string | null;
  onClose: () => void;
}

/**
 * 安装步骤组件
 */
function InstallStep({
  step,
  title,
  children,
}: {
  step: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-base">
      <div className="flex items-center justify-center size-6 rounded-full bg-brand text-white text-xs font-bold shrink-0 mt-0.5">
        {step}
      </div>
      <div className="flex-1 min-w-0">
        <h3 className="text-sm font-medium text-normal mb-2">{title}</h3>
        {children}
      </div>
    </div>
  );
}

/** 环境变量表格数据 */
const ENV_VARS = [
  {
    name: 'CENTER_URL',
    required: true,
    desc: '中心服务器 WebSocket 地址',
  },
  {
    name: 'AGENT_TOKEN',
    required: true,
    desc: '服务器创建时生成的认证 Token',
  },
  {
    name: 'SERVER_HOST',
    required: false,
    desc: '绑定的主机地址（默认自动检测）',
  },
  {
    name: 'SERVER_PORT',
    required: false,
    desc: 'Agent 监听端口（默认 9999）',
  },
  {
    name: 'LOG_LEVEL',
    required: false,
    desc: '日志级别: debug/info/warn/error',
  },
  {
    name: 'HEARTBEAT_INTERVAL',
    required: false,
    desc: '心跳间隔秒数（默认 30）',
  },
] as const;

/** FAQ 列表 */
const FAQ_ITEMS = [
  {
    q: 'Agent 无法连接到中心服务器',
    a: '检查 CENTER_URL 是否正确，确认防火墙允许 WebSocket 连接（通常是 443 或 80 端口）',
  },
  {
    q: '认证失败（Token 无效）',
    a: '确认使用的是创建服务器时生成的 Token。如已丢失，可在详情页重新生成',
  },
  {
    q: '容器启动后立即退出',
    a: '使用 docker logs 查看错误信息，检查必填环境变量是否齐全',
  },
  {
    q: '执行器未被发现',
    a: '确保 AI 工具（Claude Code/Gemini CLI）已在容器中安装并配置正确路径',
  },
] as const;

/**
 * 生成容器名称（从服务器名称派生，处理非 ASCII 字符）
 */
function toContainerName(serverName: string): string {
  const sanitized = serverName
    .toLowerCase()
    .replace(/[^a-z0-9]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
  return sanitized || 'default';
}

export function AgentInstallGuide({
  serverName,
  serverHost,
  serverPort,
  agentToken,
  dockerCommand,
  onClose,
}: AgentInstallGuideProps) {
  const containerName = toContainerName(serverName);

  /** 默认的 Docker 运行命令 */
  const defaultDockerCommand = dockerCommand ||
    `docker run -d \\
  --name vibe-agent-${containerName} \\
  --restart unless-stopped \\
  -e CENTER_URL=<你的中心服务器地址> \\
  -e AGENT_TOKEN=<你的 Agent Token> \\
  -e SERVER_HOST=${serverHost} \\
  -e SERVER_PORT=${serverPort} \\
  -v /path/to/repos:/app/workspaces \\
  -v /path/to/config:/app/config \\
  vibe-agent-daemon:latest`;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      role="dialog"
      aria-modal="true"
      aria-label="Agent 安装指引"
    >
      {/* 背景遮罩：点击关闭 */}
      <div className="absolute inset-0" onClick={onClose} />
      <div className="relative bg-panel rounded-sm border border-border w-full max-w-2xl max-h-[80vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-base py-base border-b border-border">
          <div className="flex items-center gap-2">
            <TerminalIcon className="size-5 text-brand" weight="duotone" />
            <h2 className="text-lg font-semibold text-normal">
              Agent 安装指引
            </h2>
          </div>
          <Button variant="ghost" size="sm" onClick={onClose}>
            <XIcon className="size-4" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-base space-y-base">
          {/* 服务器信息 */}
          <div className="bg-secondary rounded-sm p-base">
            <div className="text-sm text-low mb-1">目标服务器</div>
            <div className="text-base font-medium text-normal">
              {serverName}
            </div>
            <div className="text-sm text-low font-mono">
              {serverHost}:{serverPort}
            </div>
          </div>

          {/* 环境要求 */}
          <div className="flex items-start gap-2 p-base bg-warning/10 rounded-sm">
            <InfoIcon className="size-4 text-warning shrink-0 mt-0.5" />
            <div className="text-sm text-normal">
              <strong>环境要求：</strong>
              Docker 20.10+ 或 Podman 3.0+。确保目标服务器可以访问中心服务器的网络地址。
            </div>
          </div>

          {/* Agent Token */}
          {agentToken && (
            <div className="space-y-2">
              <div className="flex items-start gap-2 p-base bg-error/10 rounded-sm">
                <WarningIcon className="size-4 text-error shrink-0 mt-0.5" />
                <div className="text-sm text-normal">
                  <strong>重要：</strong>
                  以下 Agent Token 仅显示一次，请妥善保存。
                </div>
              </div>
              <CopyableCodeBlock code={agentToken} label="Agent Token" />
            </div>
          )}

          {/* 安装步骤 */}
          <div className="space-y-base">
            <h3 className="text-base font-semibold text-normal">安装步骤</h3>

            <InstallStep step={1} title="确认 Docker 已安装">
              <CopyableCodeBlock code="docker --version" />
              <p className="text-xs text-low mt-2">
                如未安装，请参考{' '}
                <a
                  href="https://docs.docker.com/get-docker/"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-brand hover:underline"
                >
                  Docker 官方安装指南
                </a>
              </p>
            </InstallStep>

            <InstallStep step={2} title="运行 Agent Daemon 容器">
              <CopyableCodeBlock
                code={defaultDockerCommand}
                label="Docker 启动命令"
              />
              <p className="text-xs text-low mt-2">
                请替换 CENTER_URL 和 AGENT_TOKEN 为实际值，并调整挂载路径
              </p>
            </InstallStep>

            <InstallStep step={3} title="验证 Agent 连接">
              <CopyableCodeBlock code={`docker logs vibe-agent-${containerName} --tail 20`} />
              <p className="text-xs text-low mt-2">
                查看日志确认 Agent 已成功连接到中心服务器
              </p>
            </InstallStep>
          </div>

          {/* 环境变量说明 */}
          <div className="space-y-2">
            <h3 className="text-base font-semibold text-normal">
              环境变量说明
            </h3>
            <div className="bg-secondary rounded-sm border border-border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left p-2 text-low font-medium">
                      变量名
                    </th>
                    <th className="text-left p-2 text-low font-medium">
                      必填
                    </th>
                    <th className="text-left p-2 text-low font-medium">
                      说明
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {ENV_VARS.map((env) => (
                    <tr key={env.name} className="border-b border-border last:border-0">
                      <td className="p-2 font-mono text-xs text-normal">
                        {env.name}
                      </td>
                      <td className="p-2">
                        <span
                          className={cn(
                            'text-xs',
                            env.required ? 'text-error' : 'text-low'
                          )}
                        >
                          {env.required ? '是' : '否'}
                        </span>
                      </td>
                      <td className="p-2 text-xs text-low">{env.desc}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* 常见问题 */}
          <div className="space-y-2">
            <h3 className="text-base font-semibold text-normal">
              常见问题排查
            </h3>
            <div className="space-y-2">
              {FAQ_ITEMS.map((faq) => (
                <div
                  key={faq.q}
                  className="bg-secondary rounded-sm p-base"
                >
                  <div className="text-sm font-medium text-normal mb-1">
                    {faq.q}
                  </div>
                  <div className="text-xs text-low">{faq.a}</div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-base py-base border-t border-border flex justify-end">
          <Button onClick={onClose}>关闭</Button>
        </div>
      </div>
    </div>
  );
}
