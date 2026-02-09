/**
 * ServerDetailPage - 服务器详情页面
 *
 * 轻量级页面组件，将渲染委托给 ServerDetailContainer。
 */
import { ServerDetailContainer } from '@/components/ui-new/containers/ServerDetailContainer';

export function ServerDetailPage() {
  return <ServerDetailContainer />;
}
