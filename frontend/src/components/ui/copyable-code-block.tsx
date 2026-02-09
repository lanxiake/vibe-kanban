/**
 * CopyableCodeBlock - 可复制的代码块组件
 *
 * 显示代码内容并提供一键复制功能。
 */
import { useState, useCallback } from 'react';
import { CheckCircleIcon, CopyIcon } from '@phosphor-icons/react';
import { Button } from '@/components/ui/button';

interface CopyableCodeBlockProps {
  code: string;
  label?: string;
}

export function CopyableCodeBlock({ code, label }: CopyableCodeBlockProps) {
  const [copied, setCopied] = useState(false);

  /** 复制代码到剪贴板 */
  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard API 可能在非 HTTPS 环境下失败，静默忽略
    }
  }, [code]);

  return (
    <div>
      {label && (
        <div className="text-xs text-low mb-1">{label}</div>
      )}
      <div className="relative">
        <div className="bg-primary rounded border border-border p-base pr-10 font-mono text-xs text-normal overflow-x-auto whitespace-pre-wrap break-all">
          {code}
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="absolute top-1 right-1"
          onClick={handleCopy}
        >
          {copied ? (
            <CheckCircleIcon className="size-4 text-success" />
          ) : (
            <CopyIcon className="size-4" />
          )}
        </Button>
      </div>
    </div>
  );
}
