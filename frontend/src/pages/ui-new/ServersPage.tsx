/**
 * ServersPage - 服务器管理页面
 *
 * 轻量级页面包装器，实际逻辑在 ServersContainer 中。
 */
import { ServersContainer } from '@/components/ui-new/containers/ServersContainer';

export function ServersPage() {
  return <ServersContainer />;
}
