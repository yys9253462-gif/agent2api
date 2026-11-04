import { Alert, Button, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@ui'
import { t } from '../ui-kit/src/i18n'
import type { ApiField, ApiSection } from './docs-api-content'
import type { ApiExample } from './docs-api-examples'

type Copy = (text: string, label: string) => void | Promise<void>

function Example({ example, onCopy }: { example: ApiExample; onCopy: Copy }) {
  return <div className='docs-example min-w-0 overflow-hidden rounded-md border border-border'>
    <div className='docs-example-head flex items-center justify-between gap-2 border-b border-border bg-surface px-3 py-2'>
      <h4 className='m-0 text-[12px] font-medium'>{example.label}</h4>
      <Button variant='ghost' size='sm' className='shrink-0' onClick={() => void onCopy(example.code, example.label)}>
        {t('复制示例')}
      </Button>
    </div>
    <pre className='m-0 max-h-[440px] overflow-auto bg-surface-inset p-3 text-[11.5px] leading-[1.7] text-foreground'
      tabIndex={0} aria-label={example.label}>
      <code className='font-mono' data-language={example.language}>{example.code}</code>
    </pre>
  </div>
}

function Fields({ fields }: { fields: ApiField[] }) {
  return <Table className='text-[12px]' aria-label={t('字段与限制')}>
    <TableHeader><TableRow>
      <TableHead className='min-w-[145px]'>{t('字段 / 状态')}</TableHead>
      <TableHead className='min-w-[110px]'>{t('类型 / 范围')}</TableHead>
      <TableHead className='min-w-[230px]'>{t('说明')}</TableHead>
    </TableRow></TableHeader>
    <TableBody>{fields.map(field => <TableRow key={field.name}>
      <TableCell className='align-top'><code className='break-words font-mono text-[11.5px]'>{field.name}</code></TableCell>
      <TableCell className='align-top text-[11.5px] text-muted-foreground'>{field.type}</TableCell>
      <TableCell className='align-top leading-relaxed text-subtle'>{field.description}</TableCell>
    </TableRow>)}</TableBody>
  </Table>
}

export function ApiReference({ section, onCopy }: { section: ApiSection; onCopy: Copy }) {
  return <div className='docs-reference' id={`docs-api-${section.id}`}>
    {section.paragraphs?.length ? <div className='docs-prose'>
      {section.paragraphs.map(text => <p key={text}>{text}</p>)}
    </div> : null}
    {section.fields?.length ? <section className='docs-block' aria-labelledby={`docs-api-${section.id}-fields`}>
      <h3 id={`docs-api-${section.id}-fields`}>{t('字段与限制')}</h3>
      <Fields fields={section.fields} />
    </section> : null}
    {section.warning ? <Alert variant='warning' className='text-[12px] leading-relaxed'>{section.warning}</Alert> : null}
    {section.points?.length ? <section className='docs-block' aria-labelledby={`docs-api-${section.id}-notes`}>
      <h3 id={`docs-api-${section.id}-notes`}>{t('使用说明与兼容边界')}</h3>
      <ul className='docs-points'>
        {section.points.map(text => <li key={text}>{text}</li>)}
      </ul>
    </section> : null}
    {section.examples?.length ? <section className='docs-block' aria-labelledby={`docs-api-${section.id}-examples`}>
      <h3 id={`docs-api-${section.id}-examples`}>{t('请求与响应示例')}</h3>
      <p className='docs-example-note'>{t('示例仅供复制，不会在本页发起请求或消耗额度。')}</p>
      {section.examples.map(example => <Example key={example.label} example={example} onCopy={onCopy} />)}
    </section> : null}
  </div>
}
