import type { ProviderConfig } from './add-account-configs'
import { t } from '../i18n'

export const DOUBAOWORK: ProviderConfig = {
  provider: 'doubaowork',
  label: t('豆包工作'),
  desktop: true,
  webLogin: {
    noteHtml: t('在豆包官方页面完成登录，窗口会自动读取完整 Cookie 并加入账号列表；不需要安装豆包工作客户端。'),
    button: t('打开网页登录'),
    hint: t('内嵌窗口打开；完成后自动加入列表，关窗即取消等待'),
    busyText: t('等待豆包工作登录完成…'),
    webHint: t('浏览器面板无法读取其它站点的 Cookie，请登录 www.doubao.com 后，使用「填写凭证」粘贴完整 Cookie。'),
  },
  manualTitle: t('粘贴 Cookie'),
  manualNote: t('粘贴 www.doubao.com 的完整 Cookie（必须含 sessionid），也支持 Cookie JSON 数组或对象。复用豆包工作账号的额度；Cookie 失效后需重新登录，没有自动续期。'),
  fields: [
    { key: 'cookie', label: 'Cookie', rows: 5, placeholder: 'sessionid=…; sessionid_ss=…; ttwid=…' },
    { key: 'name', label: t('备注名'), optional: true, placeholder: t('可选，留空则用账号昵称') },
  ],
  desktopNote: t('读取本机已登录豆包工作客户端的 Cookie，每次请求实时读取；客户端需使用 --remote-debugging-port=9342 启动。本软件不修改、关闭或重启客户端。'),
  desktopHint: t('未开启客户端调试端口时，请使用网页登录或粘贴 Cookie。'),
  fileImport: {
    accept: '.json,application/json',
    note: t('选择包含 Cookie 的 JSON 对象，或本软件导出的单账号 JSON 文件。导入后可独立使用，不需要持续运行豆包工作客户端。'),
  },
}
